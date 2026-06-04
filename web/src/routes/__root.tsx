import { HeadContent, Scripts, Link, createRootRoute } from "@tanstack/react-router";

import appCss from "../styles.css?url";
import { Navbar } from "#/components/navbar";
import { Footer } from "#/components/footer";
import { Home, BookOpen } from "lucide-react";
import { Button } from "#/components/ui/button";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "gpui-query - Async State Management for GPUI" },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      {
        rel: "preconnect",
        href: "https://fonts.googleapis.com",
      },
      {
        rel: "preconnect",
        href: "https://fonts.gstatic.com",
        crossOrigin: "anonymous",
      },
      {
        rel: "stylesheet",
        href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap",
      },
    ],
    scripts: [
      {
        children: `(function(){try{var t=localStorage.getItem('theme');if(t==='dark'||(!t&&window.matchMedia('(prefers-color-scheme:dark)').matches)){document.documentElement.classList.add('dark')}}catch(e){}})()`,
      },
    ],
  }),
  shellComponent: RootDocument,
  notFoundComponent: NotFoundPage,
});

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body>
        <a href="#main-content" className="skip-to-content">
          Skip to content
        </a>
        <div id="app" className="flex min-h-screen flex-col">
          <Navbar />
          <main id="main-content" className="flex-1">
            {children}
          </main>
          <Footer />
        </div>
        {import.meta.env.DEV && <DevTools />}
        <Scripts />
      </body>
    </html>
  );
}

function DevTools() {
  return null;
}

function NotFoundPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="flex min-h-[60vh] flex-col items-center justify-center text-center">
        <h1 className="text-8xl font-extrabold tracking-tighter text-primary">404</h1>
        <p className="mt-4 text-2xl font-semibold text-foreground">Page not found</p>
        <p className="mt-2 max-w-md text-muted-foreground">
          The page you are looking for does not exist or has been moved.
        </p>
        <div className="mt-8 flex flex-col items-center gap-3 sm:flex-row">
          <Button asChild>
            <Link to="/">
              <Home className="mr-1 h-4 w-4" />
              Go Home
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link to="/docs/$slug" params={{ slug: "getting-started" }}>
              <BookOpen className="mr-1 h-4 w-4" />
              Documentation
            </Link>
          </Button>
        </div>
      </div>
    </div>
  );
}
