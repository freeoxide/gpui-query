import { createFileRoute } from "@tanstack/react-router";

const title = "Privacy Policy - gpui-query";
const description =
  "How the gpui-query website handles data: Firebase Analytics, Google Fonts, Cloudflare hosting, and local storage.";
const url = "https://gpui-query.freeoxide.com/privacy";

export const Route = createFileRoute("/privacy")({
  head: () => ({
    meta: [
      { title },
      { name: "description", content: description },
      { property: "og:title", content: title },
      { property: "og:description", content: description },
      { property: "og:type", content: "website" },
      { property: "og:url", content: url },
    ],
    links: [{ rel: "canonical", href: url }],
  }),
  component: PrivacyPage,
});

function PrivacyPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-8 sm:px-6 sm:py-16 lg:px-8">
      <header className="border-l-4 border-primary pl-5">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Privacy Policy</h1>
        <p className="mt-2 text-muted-foreground">Last updated: July 9, 2026</p>
      </header>

      <article className="prose prose-sm mt-10 max-w-none dark:prose-invert sm:prose-base">
        <p>
          This policy covers the gpui-query website at{" "}
          <a href="https://gpui-query.freeoxide.com">gpui-query.freeoxide.com</a>, including the
          documentation and blog. The site has no user accounts, no comment forms, and no
          newsletter. We do not ask you for personal information anywhere on it. The data described
          below is what the services we build on collect.
        </p>

        <h2>Analytics</h2>
        <p>
          We use <strong>Firebase Analytics</strong> (part of Google Firebase, operated by Google
          LLC) to understand how the site is used — which pages are visited, roughly where visitors
          come from, and what kind of device and browser they use. Firebase collects this through a
          device identifier and reports it to us only in aggregate. We use it to see which docs
          pages get traffic and where readers drop off, nothing else.
        </p>
        <p>
          Google's own use of this data is described in the{" "}
          <a
            href="https://firebase.google.com/support/privacy"
            target="_blank"
            rel="noopener noreferrer"
          >
            Firebase privacy documentation
          </a>{" "}
          and the{" "}
          <a href="https://policies.google.com/privacy" target="_blank" rel="noopener noreferrer">
            Google Privacy Policy
          </a>
          . Most content blockers block Firebase Analytics; the site works exactly the same with it
          blocked.
        </p>

        <h2>Hosting</h2>
        <p>
          The site is served by <strong>Cloudflare</strong>. Like any web host, Cloudflare sees your
          IP address and request details and keeps short-lived server logs for security and
          operations. See the{" "}
          <a
            href="https://www.cloudflare.com/privacypolicy/"
            target="_blank"
            rel="noopener noreferrer"
          >
            Cloudflare Privacy Policy
          </a>
          .
        </p>

        <h2>Fonts</h2>
        <p>
          Fonts are loaded from <strong>Google Fonts</strong>. When your browser fetches a font
          file, your IP address is sent to Google as part of that request. Details are in Google's{" "}
          <a
            href="https://developers.google.com/fonts/faq/privacy"
            target="_blank"
            rel="noopener noreferrer"
          >
            Google Fonts privacy FAQ
          </a>
          .
        </p>

        <h2>Local storage</h2>
        <p>
          Your light/dark theme choice is saved in your browser's <code>localStorage</code>. It
          never leaves your device and you can clear it at any time through your browser settings.
        </p>

        <h2>External links</h2>
        <p>
          The site links out to GitHub, crates.io, docs.rs, and other third-party sites. Once you
          follow one of those links, that site's privacy policy applies, not this one.
        </p>

        <h2>Children</h2>
        <p>
          The site is developer documentation and is not directed at children under 13. We do not
          knowingly collect any information from them.
        </p>

        <h2>Changes</h2>
        <p>
          If we change what the site collects — for example by adding or removing a service — we
          will update this page and the date at the top.
        </p>

        <h2>Contact</h2>
        <p>
          Questions about this policy? Open an issue on{" "}
          <a
            href="https://github.com/freeoxide/gpui-query/issues"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>{" "}
          or email <a href="mailto:hmziqrs@gmail.com">hmziqrs@gmail.com</a>.
        </p>
      </article>
    </div>
  );
}
