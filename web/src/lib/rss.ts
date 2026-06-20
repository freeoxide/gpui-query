import { getAllBlogPosts, type BlogPost } from "./blog";

export function generateRssFeed(): string {
  // getAllBlogPosts() is already sorted by date descending.
  const posts: BlogPost[] = getAllBlogPosts();
  const siteUrl = "https://gpui-query.hmziq.xyz";

  const items = posts
    .map(
      (post) => `    <item>
      <title><![CDATA[${post.frontmatter.title}]]></title>
      <link>${siteUrl}/blog/${post.slug}</link>
      <guid isPermaLink="true">${siteUrl}/blog/${post.slug}</guid>
      <description><![CDATA[${post.frontmatter.description}]]></description>
      <pubDate>${new Date(post.frontmatter.date).toUTCString()}</pubDate>
    </item>`,
    )
    .join("\n");

  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>gpui-query Blog</title>
    <link>${siteUrl}/blog</link>
    <description>Async state management for GPUI</description>
    <language>en-us</language>
    <atom:link href="${siteUrl}/rss.xml" rel="self" type="application/rss+xml" />
    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
    <ttl>60</ttl>
${items}
  </channel>
</rss>`;
}
