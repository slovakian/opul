use super::*;

impl OpulStateMachine {
    pub fn open(config: OpulMachineConfig) -> Result<Arc<Self>, StateMachineError> {
        let support_directory = PathBuf::from(&config.application_support_folder);
        let actor = Arc::new(
            ApplicationActor::open(support_directory.clone(), config.downloads_folder)
                .map_err(open_error)?,
        );
        let torrent = TorrentEngine::new_quiescent(config.torrent).map_err(engine_error)?;
        let video_tools = config.media.video_tools.clone();
        let media = MediaEngine::new_quiescent(config.media).map_err(engine_error)?;
        let machine = Arc::new(Self {
            actor,
            torrent,
            media,
            video_tools,
            guided: Arc::new(Mutex::new(HashMap::new())),
            platform_effects: Arc::new(Mutex::new(Vec::new())),
            stop: Arc::new(AtomicBool::new(false)),
            execution_gate: Arc::new(Mutex::new(())),
            runner: Mutex::new(None),
            support_directory,
        });
        machine.sync_video_tools()?;
        machine.start_runner()?;
        if matches!(machine.actor.snapshot().session, SessionPhase::Reconciling) {
            machine.reconcile_bindings()?;
            let _ = machine.dispatch(None, Command::MarkReconciled)?;
        }
        Ok(machine)
    }

    pub fn snapshot(&self) -> MachineSnapshot {
        snapshot_view(self.actor.snapshot())
    }

    pub fn workspace_draft(&self) -> Result<MachineWorkspaceDraft, StateMachineError> {
        let id = WorkspaceId::new();
        Ok(MachineWorkspaceDraft {
            workspace_id: id.to_string(),
            folder_id: FolderId::new().to_string(),
            appearance: self
                .actor
                .propose_workspace_appearance(id)
                .map_err(actor_error)?,
        })
    }

    pub fn wait_for_snapshot(
        &self,
        after_boot_id: Option<String>,
        after_sequence: u64,
        timeout_milliseconds: u32,
    ) -> MachineSnapshot {
        let after_boot = after_boot_id
            .and_then(|value| Uuid::parse_str(&value).ok())
            .map(BootId);
        snapshot_view(self.actor.wait_for_projection(
            after_boot,
            after_sequence,
            Duration::from_millis(u64::from(timeout_milliseconds.min(30_000))),
        ))
    }

    pub fn acknowledge_recovery(&self) -> Result<MachineCommandReceipt, StateMachineError> {
        let receipt = self.dispatch(None, Command::AcknowledgeRecovery)?;
        if matches!(self.actor.snapshot().session, SessionPhase::Reconciling) {
            self.reconcile_bindings()?;
            let _ = self.dispatch(None, Command::MarkReconciled)?;
        }
        Ok(receipt)
    }

    pub fn create_workspace(
        &self,
        request: MachineWorkspaceRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let workspace_id = workspace_id(&request.workspace_id)?;
        let folder_id = folder_id(&request.folder_id)?;
        let command = Command::CreateWorkspace {
            workspace: Workspace {
                id: workspace_id,
                name: request.name,
                icon: icon_from_view(request.icon),
                appearance: request.appearance,
                folders: vec![SavedFolder {
                    id: folder_id,
                    name: request.folder_name,
                    path: request.folder_path,
                }],
                default_folder: folder_id,
                default_format: format_from_view(request.default_format),
                file_organization: organization_from_view(request.file_organization),
                view: WorkspaceView::default(),
                downloads: Vec::new(),
            },
        };
        self.dispatch_request(request.request_id, request.expected_revision, command)
    }

    pub fn update_workspace(
        &self,
        request: MachineWorkspaceRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::RenameWorkspace {
                workspace: workspace_id(&request.workspace_id)?,
                name: request.name,
                icon: icon_from_view(request.icon),
                organization: organization_from_view(request.file_organization),
            },
        )
    }

    pub fn workspace_appearance_draft(
        &self,
        workspace_id_value: String,
    ) -> Result<WorkspaceAppearance, StateMachineError> {
        let id = workspace_id(&workspace_id_value)?;
        let snapshot = self.actor.snapshot();
        let workspace = snapshot
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .ok_or_else(|| StateMachineError::Rejected {
                reason: "That workspace no longer exists.".into(),
            })?;
        let mut appearance = workspace.appearance.clone();
        if appearance.automatic_folder.is_none() {
            appearance.automatic_folder = self
                .actor
                .propose_workspace_appearance(id)
                .map_err(actor_error)?
                .automatic_folder;
        }
        Ok(appearance)
    }

    pub fn set_workspace_appearance(
        &self,
        request: MachineWorkspaceAppearanceRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            None,
            Command::SetWorkspaceAppearance {
                workspace: workspace_id(&request.workspace_id)?,
                emoji: request.emoji,
                appearance: request.appearance,
            },
        )
    }

    pub fn delete_workspace(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        workspace_id_value: String,
        replacement: Option<MachineWorkspaceRequest>,
        move_files_to_trash: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let replacement = replacement
            .map(|request| {
                let workspace_id = workspace_id(&request.workspace_id)?;
                let folder_id = folder_id(&request.folder_id)?;
                Ok(Workspace {
                    id: workspace_id,
                    name: request.name,
                    icon: icon_from_view(request.icon),
                    appearance: request.appearance,
                    folders: vec![SavedFolder {
                        id: folder_id,
                        name: request.folder_name,
                        path: request.folder_path,
                    }],
                    default_folder: folder_id,
                    default_format: format_from_view(request.default_format),
                    file_organization: organization_from_view(request.file_organization),
                    view: WorkspaceView::default(),
                    downloads: Vec::new(),
                })
            })
            .transpose()?;
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::DeleteWorkspace {
                workspace: workspace_id(&workspace_id_value)?,
                replacement,
                move_files_to_trash,
            },
        )
    }

    pub fn add_folder(
        &self,
        request: MachineFolderRequest,
        make_default: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: SavedFolder {
                    id: folder_id(&request.folder_id)?,
                    name: request.name,
                    path: request.path,
                },
                make_default,
            },
        )
    }

    pub fn set_default_folder(
        &self,
        request: MachineFolderRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::SetDefaultFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: folder_id(&request.folder_id)?,
            },
        )
    }

    pub fn relink_folder(
        &self,
        request: MachineFolderRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::RelinkFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: folder_id(&request.folder_id)?,
                name: request.name,
                path: request.path,
            },
        )
    }

    pub fn remove_folder(
        &self,
        request: MachineFolderRequest,
        move_files_to_trash: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::RemoveFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: folder_id(&request.folder_id)?,
                move_files_to_trash,
            },
        )
    }

    pub fn set_workspace_view(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        workspace_id_value: String,
        layout: MachineLayout,
        sort: MachineSort,
        grouping: MachineGrouping,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::SetWorkspaceView {
                workspace: workspace_id(&workspace_id_value)?,
                view: WorkspaceView {
                    layout: match layout {
                        MachineLayout::List => WorkspaceLayout::List,
                        MachineLayout::Table => WorkspaceLayout::Table,
                    },
                    sort: match sort {
                        MachineSort::Order => WorkspaceSort::Order,
                        MachineSort::Name => WorkspaceSort::Name,
                        MachineSort::State => WorkspaceSort::State,
                        MachineSort::Source => WorkspaceSort::Source,
                    },
                    grouping: match grouping {
                        MachineGrouping::None => WorkspaceGrouping::None,
                        MachineGrouping::State => WorkspaceGrouping::State,
                        MachineGrouping::Folder => WorkspaceGrouping::Folder,
                        MachineGrouping::Kind => WorkspaceGrouping::Kind,
                        MachineGrouping::Source => WorkspaceGrouping::Source,
                    },
                },
            },
        )
    }

    pub fn set_default_format(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        workspace_id_value: String,
        format: MachineFormat,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::SetDefaultFormat {
                workspace: workspace_id(&workspace_id_value)?,
                format: format_from_view(format),
            },
        )
    }
}
