import type { CatalogEpisodeGroup, CatalogItemType, UnifiedEpisode } from '@/types'

export function extractEpisodeIndex(label: string): number | null {
  const trimmed = label.trim()

  const chineseMatch = trimmed.match(/第\s*(\d+)\s*[集期]/)
  if (chineseMatch) return parseInt(chineseMatch[1], 10)

  const seasonMatch = trimmed.match(/S\d+E(\d+)/i)
  if (seasonMatch) return parseInt(seasonMatch[1], 10)

  const epMatch = trimmed.match(/^E(\d+)$/i)
  if (epMatch) return parseInt(epMatch[1], 10)

  const pureNum = trimmed.match(/^(\d+)$/)
  if (pureNum) return parseInt(pureNum[1], 10)

  return null
}

export function formatDisplayLabel(original: string, itemType?: CatalogItemType): string {
  const idx = extractEpisodeIndex(original)
  if (idx === null) return original
  const unit = itemType === 'variety' ? '期' : '集'
  return `第${idx}${unit}`
}

export function mergeEpisodes(
  groups: CatalogEpisodeGroup[],
  itemType: CatalogItemType
): UnifiedEpisode[] {
  if (itemType === 'movie') {
    return groups.flatMap(g =>
      g.episodes.map(ep => ({
        normalizedIndex: ep.id,
        displayLabel: ep.episode_label,
        sources: [{
          sourceKey: g.source_name,
          sourceName: g.source_name,
          lineName: (ep as any).meta ?? undefined,
          episode: ep,
        }],
      }))
    )
  }

  const map = new Map<number, UnifiedEpisode>()

  for (const group of groups) {
    for (const ep of group.episodes) {
      const idx = extractEpisodeIndex(ep.episode_label)
      if (idx === null) {
        map.set(ep.id, {
          normalizedIndex: ep.id,
          displayLabel: ep.episode_label,
          sources: [{
            sourceKey: group.source_name,
            sourceName: group.source_name,
            lineName: (ep as any).meta ?? undefined,
            episode: ep,
          }],
        })
        continue
      }

      const existing = map.get(idx)
      if (existing) {
        existing.sources.push({
          sourceKey: group.source_name,
          sourceName: group.source_name,
          lineName: (ep as any).meta ?? undefined,
          episode: ep,
        })
      } else {
        map.set(idx, {
          normalizedIndex: idx,
          displayLabel: formatDisplayLabel(ep.episode_label, itemType),
          sources: [{
            sourceKey: group.source_name,
            sourceName: group.source_name,
            lineName: (ep as any).meta ?? undefined,
            episode: ep,
          }],
        })
      }
    }
  }

  return Array.from(map.values()).sort((a, b) => a.normalizedIndex - b.normalizedIndex)
}
