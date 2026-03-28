import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import { Changelog } from './changelog';
import { CopyPrompt } from './copy-prompt';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    Changelog,
    CopyPrompt,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
