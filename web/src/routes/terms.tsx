import { createFileRoute, Link } from "@tanstack/react-router";
import { LegalPage, LegalSection } from "#/components/legal-page";

const title = "Terms of Service - gpui-query";
const description =
  "Terms for using the gpui-query website and documentation. The gpui-query library itself is MIT licensed.";
const url = "https://gpui-query.freeoxide.com/terms";

export const Route = createFileRoute("/terms")({
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
  component: TermsPage,
});

function TermsPage() {
  return (
    <LegalPage
      title="Terms of Service"
      description="Terms for using the gpui-query website, documentation, blog, and changelog."
      updatedAt="July 9, 2026"
    >
      <LegalSection title="Scope">
        <p>
          These terms apply to the gpui-query website at{" "}
          <a href="https://gpui-query.freeoxide.com">gpui-query.freeoxide.com</a> — the landing
          pages, documentation, blog, and changelog. By using the site you agree to them. They are
          short because the site is simple: it is free documentation for an open-source library.
        </p>
      </LegalSection>

      <LegalSection title="The library is MIT licensed">
        <p>
          The gpui-query crate itself is distributed under the{" "}
          <a
            href="https://github.com/freeoxide/gpui-query/blob/master/LICENSE"
            target="_blank"
            rel="noopener noreferrer"
          >
            MIT License
          </a>
          . That license — not these terms — governs your use of the source code and the published
          crate. You can use it in personal and commercial projects, modify it, and redistribute it,
          subject to the license's conditions.
        </p>
      </LegalSection>

      <LegalSection title="Site content">
        <p>
          You may read, link to, and quote the documentation and blog posts with attribution. Code
          snippets shown in the docs and blog are provided so you can use them; treat them as
          MIT-licensed like the library they document.
        </p>
      </LegalSection>

      <LegalSection title="No warranty">
        <p>
          The site and its content are provided as-is. We work to keep the documentation accurate,
          but APIs change between releases and pages can lag behind the code. Nothing on this site
          is a guarantee that the library is fit for a particular purpose, and we are not liable for
          damages arising from your use of the site or the library — the same disclaimer the MIT
          License makes for the code.
        </p>
      </LegalSection>

      <LegalSection title="Acceptable use">
        <p>
          Don't attempt to disrupt the site, scrape it at a rate that degrades it for others, or use
          it to distribute malware or spam. That's it.
        </p>
      </LegalSection>

      <LegalSection title="Third-party services and links">
        <p>
          The site links to third-party sites (GitHub, crates.io, docs.rs, and others) and relies on
          the services described in the <Link to="/privacy">Privacy Policy</Link>, including
          Firebase Analytics, Google Fonts, and Cloudflare. We don't control those services and
          aren't responsible for their content or conduct.
        </p>
      </LegalSection>

      <LegalSection title="Changes">
        <p>
          We may update these terms as the site evolves. Material changes will be reflected on this
          page with a new date at the top. Continuing to use the site after a change means you
          accept the updated terms.
        </p>
      </LegalSection>

      <LegalSection title="Contact">
        <p>
          Questions about these terms? Open an issue on{" "}
          <a
            href="https://github.com/freeoxide/gpui-query/issues"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>{" "}
          or email <a href="mailto:hmziqrs@gmail.com">hmziqrs@gmail.com</a>.
        </p>
      </LegalSection>
    </LegalPage>
  );
}
