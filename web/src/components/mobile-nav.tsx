import { Link } from "@tanstack/react-router";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "#/components/ui/sheet";
import { GithubLogoIcon } from "@phosphor-icons/react";

const navLinks = [
  { href: "/docs/", label: "Docs" },
  { href: "/faq", label: "FAQ" },
  { href: "/about", label: "About" },
];

interface MobileNavProps {
  open: boolean;
  onClose: () => void;
}

export function MobileNav({ open, onClose }: MobileNavProps) {
  return (
    <Sheet open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <SheetContent side="left">
        <SheetHeader>
          <SheetTitle>Navigation</SheetTitle>
        </SheetHeader>
        <nav className="flex flex-col gap-4 pt-4" aria-label="Mobile navigation">
          {navLinks.map((link) => (
            <Link
              key={link.href}
              to={link.href}
              onClick={onClose}
              className="text-lg font-medium text-muted-foreground transition-colors hover:text-foreground"
            >
              {link.label}
            </Link>
          ))}
          <a
            href="https://gpui.rs/blog"
            target="_blank"
            rel="noopener noreferrer"
            className="text-lg font-medium text-muted-foreground transition-colors hover:text-foreground"
          >
            Blog
          </a>
          <a
            href="https://github.com/hmziqrs/gpui-query"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-lg font-medium text-muted-foreground transition-colors hover:text-foreground"
          >
            <GithubLogoIcon size={20} />
            GitHub
          </a>
        </nav>
      </SheetContent>
    </Sheet>
  );
}
