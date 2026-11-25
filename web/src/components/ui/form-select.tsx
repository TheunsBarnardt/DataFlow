import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

const EMPTY = "__empty__";

export type FormSelectOption = {
  value: string;
  label: string;
  /** Visual separator before this item */
  separatorBefore?: boolean;
  /** Muted / accent styling (e.g. create action) */
  accent?: boolean;
};

function toSelectValue(value: string): string | undefined {
  if (value === "") return undefined;
  return value;
}

function fromSelectValue(value: string): string {
  return value === EMPTY ? "" : value;
}

export function FormSelect({
  id,
  value,
  onChange,
  options,
  placeholder = "Select…",
  disabled,
  className,
  contentClassName,
  size = "default",
}: {
  id?: string;
  value: string;
  onChange: (next: string) => void;
  options: FormSelectOption[];
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  contentClassName?: string;
  size?: "sm" | "default";
}) {
  const selectedLabel = options.find((o) => o.value === value)?.label;

  return (
    <Select
      value={toSelectValue(value)}
      onValueChange={(next) => onChange(fromSelectValue(next))}
      disabled={disabled}
    >
      <SelectTrigger id={id} size={size} className={cn("w-full", className)}>
        <SelectValue placeholder={selectedLabel || placeholder} />
      </SelectTrigger>
      <SelectContent
        className={cn("z-[90]", contentClassName)}
        position="popper"
      >
        <SelectGroup>
          {options.flatMap((opt) => {
            const itemValue = opt.value === "" ? EMPTY : opt.value;
            const nodes = [];
            if (opt.separatorBefore) {
              nodes.push(<SelectSeparator key={`${itemValue}-sep`} />);
            }
            nodes.push(
              <SelectItem
                key={itemValue}
                value={itemValue}
                className={cn(opt.accent && "text-primary")}
              >
                {opt.label}
              </SelectItem>,
            );
            return nodes;
          })}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}
