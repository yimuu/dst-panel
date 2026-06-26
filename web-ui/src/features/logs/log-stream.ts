export function buildLogStreamUrl(origin: string, cluster: string): string {
  const url = new URL('/ws/log', origin)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  url.searchParams.set('cluster', cluster)
  return url.toString()
}
