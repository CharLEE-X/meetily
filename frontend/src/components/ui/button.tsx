import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-xl text-sm font-semibold transition-[background-color,border-color,color,box-shadow,transform] duration-200 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-700/20 focus-visible:ring-offset-2 focus-visible:ring-offset-white disabled:pointer-events-none disabled:opacity-50 active:translate-y-px [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default:
          "bg-slate-950 text-white shadow-[0_10px_24px_rgba(15,23,42,0.14)] hover:bg-slate-800",
        destructive:
          "bg-red-600 text-white shadow-[0_10px_24px_rgba(220,38,38,0.14)] hover:bg-red-700",
        outline:
          "border border-slate-200/90 bg-white/95 text-slate-700 shadow-[0_1px_2px_rgba(15,23,42,0.04)] hover:border-slate-300 hover:bg-slate-50 hover:text-slate-950",
        secondary:
          "bg-slate-100 text-slate-800 shadow-none hover:bg-slate-200",
        ghost: "text-slate-700 hover:bg-slate-100 hover:text-slate-950",
        link: "text-primary underline-offset-4 hover:underline",
        green: "bg-emerald-700 text-white shadow-[0_10px_24px_rgba(4,120,87,0.16)] hover:bg-emerald-800",
        blue: "bg-slate-900 text-white shadow-[0_10px_24px_rgba(15,23,42,0.14)] hover:bg-slate-800",
        red: "bg-red-600 text-white shadow-[0_10px_24px_rgba(220,38,38,0.14)] hover:bg-red-700",
        gray: "border border-slate-200 bg-slate-100 text-slate-700 hover:bg-slate-200 hover:text-slate-950",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-xl px-3 text-xs",
        lg: "h-10 rounded-xl px-8",
        icon: "h-9 w-9 rounded-xl",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button"
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    )
  }
)
Button.displayName = "Button"

export { Button, buttonVariants }
