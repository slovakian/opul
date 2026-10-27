import {
  Fragment,
  useRef,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";
import { Check, ChevronDown, type LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from "@/components/ui/sidebar";

export type MenuAction = {
  label: string;
  content?: ReactNode;
  icon?: LucideIcon;
  onClick: (trigger: HTMLButtonElement | null) => void;
  disabled?: boolean;
  selected?: boolean;
  hint?: string;
  destructive?: boolean;
};
type MenuGroup = { label?: string; actions: MenuAction[] };

export function SidebarPopover({
  trigger,
  title,
  groups,
  align = "end",
}: {
  trigger: ReactElement;
  title: string;
  groups: MenuGroup[];
  align?: "start" | "end";
}) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger ref={triggerRef} render={trigger} />
      <PopoverContent
        align={align}
        className="w-60 overflow-hidden p-0"
        onKeyDown={(event) => {
          if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key))
            return;
          const buttons = Array.from(
            event.currentTarget.querySelectorAll<HTMLButtonElement>(
              "button:not(:disabled)",
            ),
          );
          if (!buttons.length) return;
          event.preventDefault();
          const index = buttons.findIndex(
            (button) => button === document.activeElement,
          );
          buttons[
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? buttons.length - 1
                : (index +
                    (event.key === "ArrowUp" ? -1 : 1) +
                    buttons.length) %
                  buttons.length
          ]?.focus();
        }}
      >
        <PopoverTitle className="sr-only">{title}</PopoverTitle>
        <Sidebar collapsible="none" className="w-auto bg-transparent">
          <SidebarContent className="max-h-96 gap-0">
            {groups.map((group, index) => (
              <Fragment key={group.label ?? index}>
                {index > 0 && <SidebarSeparator />}
                <SidebarGroup>
                  {group.label && (
                    <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
                  )}
                  <SidebarMenu>
                    {group.actions.map((action) => (
                      <SidebarMenuItem key={action.label}>
                        <SidebarMenuButton
                          type="button"
                          aria-label={action.label}
                          disabled={action.disabled}
                          isActive={action.selected}
                          variant={
                            action.destructive ? "destructive" : "default"
                          }
                          onClick={() => {
                            setOpen(false);
                            action.onClick(triggerRef.current);
                          }}
                        >
                          {action.icon && <action.icon />}
                          <span className="flex min-w-0 items-center gap-2">
                            {action.content ?? action.label}
                          </span>
                          {action.selected ? (
                            <Check className="ml-auto" />
                          ) : (
                            action.hint && (
                              <span className="ml-auto text-xs text-muted-foreground">
                                {action.hint}
                              </span>
                            )
                          )}
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))}
                  </SidebarMenu>
                </SidebarGroup>
              </Fragment>
            ))}
          </SidebarContent>
        </Sidebar>
      </PopoverContent>
    </Popover>
  );
}

export function Choice<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string; content?: ReactNode }[];
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  const selected = options.find((option) => option.value === value);
  return (
    <SidebarPopover
      title={label}
      align="start"
      trigger={
        <Button
          type="button"
          variant="outline"
          className="w-full justify-between"
          aria-label={`${label}: ${selected?.label ?? "Choose"}`}
          disabled={disabled}
        >
          <span className="flex min-w-0 items-center gap-2">
            {selected?.content ?? (
              <span className="truncate">{selected?.label ?? "Choose…"}</span>
            )}
          </span>
          <ChevronDown data-icon="inline-end" />
        </Button>
      }
      groups={[
        {
          label,
          actions: options.map((option) => ({
            label: option.label,
            content: option.content,
            selected: value === option.value,
            onClick: () => onChange(option.value),
          })),
        },
      ]}
    />
  );
}

export function MenuTrigger({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <Button variant="ghost" size="icon-sm" aria-label={label} title={label}>
      {children}
    </Button>
  );
}
