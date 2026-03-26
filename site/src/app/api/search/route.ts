import { source } from '@/lib/source';
import { createFromSource } from 'fumadocs-core/search/server';

const server = createFromSource(source);

export const revalidate = false;

export function GET() {
  return server.staticGET();
}
