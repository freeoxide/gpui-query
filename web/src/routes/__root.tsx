import { HeadContent, Scripts, Link, createRootRoute } from "@tanstack/react-router";

import appCss from "../styles.css?url";
import { Navbar } from "#/components/navbar";
import { Footer } from "#/components/footer";
import { HouseIcon, BookOpenIcon } from "@phosphor-icons/react";
import { Button } from "#/components/ui/button";
import { FirebaseAnalytics } from "#/components/firebase-analytics";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { name: "theme-color", content: "#1E6B5A" },
      { title: "gpui-query - Async State Management for GPUI" },
    ],
    links: [
      { rel: "stylesheet", href: appCss },
      { rel: "icon", type: "image/x-icon", href: "/favicon.ico" },
      { rel: "manifest", href: "/manifest.json" },
    ],
    scripts: [],
  }),
  shellComponent: RootDocument,
  notFoundComponent: NotFoundPage,
  errorComponent: ErrorPage,
});

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html: `(function(){try{var t=localStorage.getItem('theme');if(t==='dark'||(!t&&window.matchMedia('(prefers-color-scheme:dark)').matches)){document.documentElement.classList.add('dark');document.querySelector('meta[name=theme-color]')?.setAttribute('content','#0C1222')}}catch(e){}})()`,
          }}
        />
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
        <FirebaseAnalytics />
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
              <HouseIcon size={16} className="mr-1" />
              Go Home
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <a href="/docs/">
              <BookOpenIcon size={16} className="mr-1" />
              Documentation
            </a>
          </Button>
        </div>
      </div>
    </div>
  );
}

function ErrorPage({ error }: { error: Error }) {
  return (
    <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <div className="flex min-h-[60vh] flex-col items-center justify-center text-center">
        <h1 className="text-8xl font-extrabold tracking-tighter text-destructive">Oops</h1>
        <p className="mt-4 text-2xl font-semibold text-foreground">Something went wrong</p>
        <p className="mt-2 max-w-md text-muted-foreground">
          {error.message || "An unexpected error occurred."}
        </p>
        <div className="mt-8 flex flex-col items-center gap-3 sm:flex-row">
          <Button asChild>
            <Link to="/">
              <HouseIcon size={16} className="mr-1" />
              Go Home
            </Link>
          </Button>
          <Button variant="outline" onClick={() => window.location.reload()}>
            Reload page
          </Button>
        </div>
      </div>
    </div>
  );
}
