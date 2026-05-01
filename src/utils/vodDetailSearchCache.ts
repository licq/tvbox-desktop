import type { CatalogEpisode, SearchResult } from '@/types'

export interface VodDetailProviderDetail {
  title: string | null
  poster: string | null
  summary: string | null
  episodes: CatalogEpisode[]
}

export interface VodDetailSearchGroup {
  source_name: string
  results: SearchResult[]
}

export interface VodDetailSearchSnapshot {
  searchResults: VodDetailSearchGroup[]
  providerDetailEntries: Array<[string, VodDetailProviderDetail]>
}

const searchSnapshotCache = new Map<string, VodDetailSearchSnapshot>()

export function normalizeVodDetailSearchKey(keyword: string): string {
  return keyword.trim().toLowerCase()
}

function cloneSearchResult(result: SearchResult): SearchResult {
  return { ...result }
}

function cloneProviderDetail(detail: VodDetailProviderDetail): VodDetailProviderDetail {
  return {
    title: detail.title,
    poster: detail.poster,
    summary: detail.summary,
    episodes: detail.episodes.map(episode => ({ ...episode })),
  }
}

function cloneSnapshot(snapshot: VodDetailSearchSnapshot): VodDetailSearchSnapshot {
  return {
    searchResults: snapshot.searchResults.map(group => ({
      source_name: group.source_name,
      results: group.results.map(cloneSearchResult),
    })),
    providerDetailEntries: snapshot.providerDetailEntries.map(([key, detail]) => [
      key,
      cloneProviderDetail(detail),
    ]),
  }
}

export function getVodDetailSearchSnapshot(keyword: string): VodDetailSearchSnapshot | null {
  const cached = searchSnapshotCache.get(normalizeVodDetailSearchKey(keyword))
  return cached ? cloneSnapshot(cached) : null
}

export function setVodDetailSearchSnapshot(keyword: string, snapshot: VodDetailSearchSnapshot): void {
  searchSnapshotCache.set(normalizeVodDetailSearchKey(keyword), cloneSnapshot(snapshot))
}

export function clearVodDetailSearchSnapshot(keyword?: string): void {
  if (keyword) {
    searchSnapshotCache.delete(normalizeVodDetailSearchKey(keyword))
    return
  }

  searchSnapshotCache.clear()
}
