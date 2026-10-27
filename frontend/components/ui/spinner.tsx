import { cn } from "cn";
import { Loader2Icon } from "lucide-react";

const Spinner = ({ className, ...props }: React.ComponentProps<"svg">) => (
  <output aria-label="Loading" className="inline-flex">
    <Loader2Icon
      data-slot="spinner"
      className={cn("size-4 animate-spin", className)}
      {...props}
    />
  </output>
);

export { Spinner };
