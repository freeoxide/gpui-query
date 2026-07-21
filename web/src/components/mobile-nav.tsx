import { Sheet, SheetContent, SheetHeader, SheetTitle } from "#/components/ui/sheet";
import { GithubLogoIcon } from "@phosphor-icons/react";

const navLinks = [
  { href: "/docs/", label: "Docs" },
  { href: "/blog", label: "Blog" },
  { href: "/faq", label: "FAQ" },
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
          {navLinks.map((link) =>
            link.href.startsWith("/docs") ? (
              <a
                key={link.href}
                href={link.href}
                onClick={onClose}
                className="text-lg font-medium text-muted-foreground transition-colors hover:text-foreground"
              >
                {link.label}
              </a>
            ) : (
              <a
                key={link.href}
                href={link.href}
                onClick={onClose}
                className="text-lg font-medium text-muted-foreground transition-colors hover:text-foreground"
              >
                {link.label}
              </a>
            ),
          )}
          <a
            href="https://github.com/freeoxide/gpui-query"
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
