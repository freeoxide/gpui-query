import type {ReactNode} from 'react';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Link from '@docusaurus/Link';
import Heading from '@theme/Heading';
import Layout from '@theme/Layout';

import styles from './index.module.css';

/* ─── SVG Icons (inline, no deps) ──────────────────────────── */

function ArrowRightIcon({className}: {className?: string}) {
  return (
    <svg
      className={className}
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <path d="M5 12h14" />
      <path d="m12 5 7 7-7 7" />
    </svg>
  );
}

function GitHubIcon({className}: {className?: string}) {
  return (
    <svg
      className={className}
      xmlns="http://www.w3.org/2000/svg"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="currentColor">
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}

function TerminalIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  );
}

function CodeIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </svg>
  );
}

/* ─── Feature Icons ─────────────────────────────────────────── */

function ZapIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
    </svg>
  );
}

function ArrowRightLeftIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <polyline points="8 3 4 7 8 11" />
      <polyline points="16 3 20 7 16 11" />
      <line x1="4" y1="7" x2="20" y2="7" />
    </svg>
  );
}

function InfinityIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <path d="M12 12c-2-2.67-4-4-6-4a4 4 0 1 0 0 8c2 0 4-1.33 6-4Zm0 0c2 2.67 4 4 6 4a4 4 0 0 0 0-8c-2 0-4 1.33-6 4Z" />
    </svg>
  );
}

function DatabaseIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <ellipse cx="12" cy="5" rx="9" ry="3" />
      <path d="M3 5V19A9 3 0 0 0 21 19V5" />
      <path d="M3 12A9 3 0 0 0 21 12" />
    </svg>
  );
}

function XCircleIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <path d="m15 9-6 6" />
      <path d="m9 9 6 6" />
    </svg>
  );
}

function HardDriveIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round">
      <line x1="22" y1="12" x2="2" y2="12" />
      <path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
      <line x1="6" y1="16" x2="6.01" y2="16" />
      <line x1="10" y1="16" x2="10.01" y2="16" />
    </svg>
  );
}

/* ─── Features Data ─────────────────────────────────────────── */

const features = [
  {
    icon: <ZapIcon />,
    title: 'Declarative Queries',
    description:
      'Write queries declaratively. gpui-query handles fetching, caching, and state updates automatically.',
  },
  {
    icon: <ArrowRightLeftIcon />,
    title: 'Mutations',
    description:
      'First-class mutation support with success/error callbacks and optimistic updates.',
  },
  {
    icon: <InfinityIcon />,
    title: 'Infinite Queries',
    description:
      'Paginate effortlessly with built-in infinite query support and bidirectional fetching.',
  },
  {
    icon: <DatabaseIcon />,
    title: 'Smart Caching',
    description:
      'TTL, Stale-While-Revalidate, LatestWins, IgnoreWhileLoading — cache policies for every use case.',
  },
  {
    icon: <XCircleIcon />,
    title: 'Cancellation',
    description:
      'Signal-checked retries and cooperative cancellation via Arc<AtomicBool> for clean async lifecycle management.',
  },
  {
    icon: <HardDriveIcon />,
    title: 'Persistence',
    description:
      'Serialize and restore query state with custom persistence backends.',
  },
];

/* ─── Sections ──────────────────────────────────────────────── */

function HeroSection() {
  return (
    <section className={styles.heroSection}>
      <div className={styles.heroDotGrid} aria-hidden="true" />
      <div className={styles.heroGlow} aria-hidden="true" />

      <div className={styles.heroInner}>
        {/* Badge */}
        <div className={styles.badge}>
          <span className={styles.pulsingDot}>
            <span />
            <span />
          </span>
          Open-source async state for Rust GPUI
        </div>

        {/* Title */}
        <Heading as="h1" className={styles.heroTitle}>
          <span className={styles.gradientText}>Zero-boilerplate</span>
          <br />
          async state for GPUI
        </Heading>

        {/* Subtitle */}
        <p className={styles.heroSubtitle}>
          Fetch, cache, and synchronize async data with a single hook. No manual
          lifecycle management. Built for the Zed editor framework.
        </p>

        {/* CTA Buttons */}
        <div className={styles.heroCtaRow}>
          <Link className={styles.buttonPrimary} to="/docs/">
            Get Started
            <ArrowRightIcon className={styles.arrowIcon} />
          </Link>
          <Link
            className={styles.buttonOutline}
            href="https://github.com/hmziqrs/gpui-query">
            <GitHubIcon className={styles.githubIcon} />
            GitHub
          </Link>
        </div>
      </div>
    </section>
  );
}

function FeatureGrid() {
  return (
    <section className={styles.featuresSection}>
      <div className={styles.featuresInner}>
        <Heading as="h2" className={styles.featuresHeading}>
          Everything you need for async state
        </Heading>
        <p className={styles.featuresSubheading}>
          A complete toolkit for managing asynchronous data flows in your GPUI
          applications.
        </p>

        <div className={styles.featuresGrid}>
          {features.map((feature) => (
            <div key={feature.title} className={styles.featureCard}>
              <div className={styles.featureIcon}>{feature.icon}</div>
              <h3 className={styles.featureTitle}>{feature.title}</h3>
              <p className={styles.featureDescription}>
                {feature.description}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function QuickStartSection() {
  return (
    <section className={styles.quickStartSection}>
      <div className={styles.quickStartInner}>
        <div className={styles.quickStartLabel}>
          <Heading as="h2" className={styles.quickStartHeading}>
            Quick Start
          </Heading>
          <p className={styles.quickStartSubheading}>
            Start fetching data in just a few lines of Rust.
          </p>
        </div>

        <div className={styles.quickStartBox}>
          <div className={styles.codeBlockLabel}>
            <TerminalIcon />
            Install the crate
          </div>
          <pre className={styles.codeBlock}>
            <code>{`[dependencies]\ngpui-query = "0.1"`}</code>
          </pre>

          <div className={styles.codeBlockLabel}>
            <CodeIcon />
            Use it in your view
          </div>
          <pre className={styles.codeBlock}>
            <code>
              {`use gpui_query::prelude::*;

fn render_user_list(cx: &mut ViewContext<App>) -> impl IntoElement {
    let query = use_query(cx, "user-list", || async {
        fetch_users().await
    });

    div().children(match &query.data {
        Some(users) => users.iter().map(|u| render_user(u)),
        None => vec![div().child("Loading...")],
    })
}`}
            </code>
          </pre>
        </div>

        <div className={styles.quickStartReadMore}>
          <Link className={styles.readMoreLink} to="/docs/">
            Read the full guide
            <ArrowRightIcon className={styles.arrowIcon} />
          </Link>
        </div>
      </div>
    </section>
  );
}

function CtaFooterSection() {
  return (
    <section className={styles.ctaSection}>
      <div className={styles.ctaDotGrid} aria-hidden="true" />
      <div className={styles.ctaInner}>
        <div className={styles.ctaBox}>
          <Heading as="h2" className={styles.ctaHeading}>
            Ready to simplify async state?
          </Heading>
          <p className={styles.ctaSubtitle}>
            Get started with gpui-query in minutes and focus on building great
            applications, not managing async boilerplate.
          </p>

          <div className={styles.ctaButtons}>
            <Link className={styles.buttonPrimary} to="/docs/">
              Get Started
              <ArrowRightIcon className={styles.arrowIcon} />
            </Link>
            <Link
              className={styles.buttonOutline}
              href="https://github.com/hmziqrs/gpui-query">
              <GitHubIcon className={styles.githubIcon} />
              View on GitHub
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
}

/* ─── Page ──────────────────────────────────────────────────── */

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title="gpui-query — Async State Management for GPUI"
      description="Zero-boilerplate async state management for GPUI. Caching, retry, cooperative cancellation, and persistence for the Zed editor's framework.">
      <main>
        <HeroSection />
        <FeatureGrid />
        <QuickStartSection />
        <CtaFooterSection />
      </main>
    </Layout>
  );
}
