# 0024: A Rust application state machine owns app state

Status: accepted

Opul has one process-wide application state machine in `opul_core`. It is the sole authority for Workspaces, Saved Folders, Downloads, transfer intent, transfer lifecycle, recovery identity, and shutdown intent. SwiftUI renders immutable projections, submits typed commands, and owns only transient presentation and macOS-only capabilities such as AppKit, WebKit, and the pasteboard.

This supersedes ADR 0021's ownership boundary. Its media execution and security decisions remain requirements, but the Rust media queue is now an effect adapter beneath the application state machine rather than an independent source of Download truth. This also clarifies ADR 0020: librqbit remains the torrent engine, while its session persistence is an operational adapter checkpoint rather than the product model.

## The model

The state-machine kernel is a pure, source-specific reducer. Torrent and media carry different source data, and the aggregate validates source-specific lifecycle rules at every construction, load, migration, and transition boundary, so a media Download cannot seed and a torrent cannot enter FFmpeg processing. A stable `DownloadId` is allocated before execution and survives retries and restart. Backend identifiers such as an info hash or media process key are bindings, never Download identity.

Every Download is structurally contained by one Workspace and refers to one Saved Folder in that Workspace. A Workspace has a nonempty folder catalog and one separate default-folder identity. Constructors, deserialization, migration, and every command validate the global invariants that types alone cannot prove.

Rust derives each Download's capabilities from the same transition rules that accept or reject commands. Swift does not reconstruct lifecycle policy and does not optimistically assign transfer state.

## Ordering and effects

A single Rust actor orders commands, engine facts, platform-effect completions, and shutdown. It runs the pure reducer, commits the next aggregate and its outbox in one atomic checkpoint, and only then executes effects. Reads are pure and cannot reconcile or mutate state.

Effects are delivered at least once and must be idempotent. Each transfer has two distinct ordering identities:

- an incarnation, changed when retry creates a new backend attempt;
- a control generation, changed by pause, resume, cancel, removal, and shutdown intent;

Before dispatch, the actor checks that an effect still serves the current incarnation, generation, and session phase. Control effects for a transfer execute in order. If an obsolete operation is already outside Opul and cannot be cancelled, the adapter converges to the latest desired state after it completes. Rejecting a stale result is not considered sufficient if the stale operation could still affect the runtime.

The lifecycle includes truthful pending phases such as Pausing and Retiring. Removal retains the binding, destination, and output evidence until the writer has stopped, the adapter record is absent, and any requested Trash operation has been resolved. If Trash fails or output identity changed, Opul leaves the file untouched, surfaces the failure, and finishes the library removal rather than wedging the aggregate. A reused pathname is not proof that a newly appearing file is owned by the old Download.

Workspace and Saved Folder deletion are aggregate-level structural intents, not Swift retry loops. The requested structural change and every required Download retirement are committed together; the container is removed or replaced only after those retirements complete.

## Persistence and recovery

The canonical store is a versioned, checksummed, owner-only checkpoint containing the domain aggregate, command receipts, authoritative migration metadata, and pending effects. High-frequency rates, peer counts, and ETA are a volatile overlay and do not rewrite the checkpoint.

Commit writes and flushes a temporary file, atomically installs it, and flushes the containing directory. A previously validated checkpoint is kept as a backup. A fallback to an older backup is not an ordinary successful load: execution stays fenced until the recovery is explicitly acknowledged, adapters and outputs are reconciled, lost generations are surfaced, and pending effects are revalidated. Effect IDs are globally unique and are never derived only from a revision that rollback could repeat.

Legacy Swift library and engine records are imported once, before adapters start. Existing Download UUIDs remain canonical. Media records join only through exact transfer IDs and torrents through exact info hashes. Ambiguous or engine-only records become recovery items; they are never silently adopted into the first Workspace. Legacy files remain read-only recovery evidence after a successful import.

Startup opens adapters quiescent and reconciles identity before dispatching any effect that can run a transfer.

Quitting is an explicit Quiescing transition. New mutations stop, transfer adapters and owned child-process groups stop, and Stopped is committed before normal termination. Running intent is retained so the next launch can resume it; a person-held Download remains held. Closing the last window still quits Opul.

## Boundaries retained from earlier decisions

- ADR 0001: a Download belongs to exactly one Workspace.
- ADR 0002: every Workspace remains ordinary, including a replacement Downloads Workspace after deleting the last one.
- ADR 0004: quit stops transfers and a later launch resumes prior running intent.
- ADR 0005: every Download destination is one Saved Folder of its Workspace.
- ADR 0006: playlist entries become ordered individual Downloads with per-Workspace deduplication.
- ADRs 0019 and 0020: torrents are ordinary Downloads executed by one in-process librqbit session.
- ADR 0022: WebKit remains the observation mechanism for guided extraction.

## Considered options

Keeping Swift as the library owner while Rust owns two transfer queues was rejected because it preserves split commits, optimistic transitions, and heuristic recovery. Making engine stores canonical was rejected because backend identities do not encode Workspace intent. A permanent event journal was rejected because Opul needs current-state recovery, not historical queries, and event upcasters and compaction would be permanent machinery. The store remains replaceable by SQLite without changing the state-machine boundary if measurements or fault tests show that atomic checkpoints are inadequate.
