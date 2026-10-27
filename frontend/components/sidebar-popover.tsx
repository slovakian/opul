import { useState, type ReactElement, type ReactNode } from "react";
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
} from "@/components/ui/sidebar";

export type MenuAction = {
  label: string;
  icon?: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
  selected?: boolean;
  hint?: string;
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
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger render={trigger} />
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
              <SidebarGroup key={group.label ?? index}>
                {group.label && (
                  <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
                )}
                <SidebarMenu>
                  {group.actions.map((action) => (
                    <SidebarMenuItem key={action.label}>
                      <SidebarMenuButton
                        type="button"
                        disabled={action.disabled}
                        isActive={action.selected}
                        onClick={() => {
                          setOpen(false);
                          action.onClick();
                        }}
                      >
                        {action.icon && <action.icon />}
                        <span>{action.label}</span>
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
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <SidebarPopover
      title={label}
      align="start"
      trigger={
        <Button
          type="button"
          variant="outline"
          className="w-full justify-between"
          aria-label={`${label}: ${options.find((option) => option.value === value)?.label ?? "Choose"}`}
          disabled={disabled}
        >
          <span className="truncate">
            {options.find((option) => option.value === value)?.label ??
              "Choose…"}
          </span>
          <ChevronDown data-icon="inline-end" />
        </Button>
      }
      groups={[
        {
          label,
          actions: options.map((option) => ({
            label: option.label,
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
