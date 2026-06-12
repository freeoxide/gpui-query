interface SoftwareSourceCodeData {
  name: string;
  description: string;
  url: string;
  codeRepository: string;
  programmingLanguage?: string;
  license?: string;
}

interface TechArticleData {
  headline: string;
  description: string;
  author?: string;
  datePublished?: string;
  url?: string;
}

interface HowToData {
  name: string;
  description: string;
  steps: Array<{ name: string; text: string }>;
}

interface FAQData {
  questions: Array<{ question: string; answer: string }>;
}

interface BlogPostData {
  headline: string;
  description: string;
  author?: string;
  datePublished?: string;
  url?: string;
  image?: string;
}

export function softwareSourceCode(data: SoftwareSourceCodeData) {
  return {
    "@context": "https://schema.org",
    "@type": "SoftwareSourceCode",
    name: data.name,
    description: data.description,
    url: data.url,
    codeRepository: data.codeRepository,
    programmingLanguage: data.programmingLanguage ?? "Rust",
    ...(data.license && { license: data.license }),
  };
}

export function techArticle(data: TechArticleData) {
  return {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: data.headline,
    description: data.description,
    ...(data.author && { author: { "@type": "Person", name: data.author } }),
    ...(data.datePublished && { datePublished: data.datePublished }),
    ...(data.url && { url: data.url }),
  };
}

export function howTo(data: HowToData) {
  return {
    "@context": "https://schema.org",
    "@type": "HowTo",
    name: data.name,
    description: data.description,
    step: data.steps.map((step) => ({
      "@type": "HowToStep",
      name: step.name,
      text: step.text,
    })),
  };
}

export function faqPage(data: FAQData) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: data.questions.map((q) => ({
      "@type": "Question",
      name: q.question,
      acceptedAnswer: {
        "@type": "Answer",
        text: q.answer,
      },
    })),
  };
}

export function blogPost(data: BlogPostData) {
  return {
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: data.headline,
    description: data.description,
    ...(data.author && { author: { "@type": "Person", name: data.author } }),
    ...(data.datePublished && { datePublished: data.datePublished }),
    ...(data.url && { url: data.url }),
    ...(data.image && { image: data.image }),
  };
}
