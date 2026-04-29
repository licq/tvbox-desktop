export interface Subscription {
  id: number;
  name: string;
  url: string;
  enabled: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface ChannelSource {
  url: string;
  subscription_id: number;
}

export interface LiveChannel {
  id: number;
  name: string;
  logo?: string;
  category: string;
  sources: ChannelSource[];
}

export interface LiveChannelGroup {
  category: string;
  channels: LiveChannel[];
}

export interface DoubanHot {
  id: number;
  name: string;
  year: number | null;
  poster: string | null;
  rating: number | null;
  rank: number;
  updated_at: string;
  item_type: 'movie' | 'series' | 'variety' | 'anime';
}

export type SourceId = 'zxzj' | 'jpvod' | 'xb6v';

export interface SearchResult {
  source: string;
  source_name: string;
  detail_url: string;
  item_type: 'movie' | 'series' | 'variety' | 'anime' | 'generic';
  title?: string;
  poster?: string;
}

// Provider search result structure (returned by search_all_sources)
export interface SourceSearchResult {
  source_key: string
  source_name: string
  items: ProviderCatalogItem[]
}

export interface ProviderCatalogItem {
  source_item_key: string
  title: string
  item_type: string
  poster?: string
  summary?: string
  detail_json?: string
  episodes: CatalogEpisode[]
}

// Playback target (returned by provider_play)
export interface PlaybackTarget {
  episode_id: number | null
  source_key: string
  target_url: string
  target_kind: 'direct' | 'resolvable' | 'embedded' | 'external_required'
  resolver_key: string | null
  headers: Record<string, string> | null
  sort_hint: number
  meta: string | null
}

export interface VodItem {
  id: number;
  subscription_id: number;
  name: string;
  type: 'movie' | 'series' | 'variety' | 'anime';
  poster?: string;
  description?: string;
  episodes: Episode[];
}

export interface Episode {
  name: string;
  url: string;
}

export interface PlayHistory {
  id: number;
  item_type: 'live' | 'vod';
  item_id: number;
  progress: number;
  last_played: string;
}

export type SourceKind = 'simple_json' | 'tvbox_config';

export interface SourceSubscription {
  id: number;
  name: string;
  url: string;
  kind: SourceKind;
  enabled: boolean;
  last_refreshed_at?: string;
  last_error?: string | null;
}

export type CatalogItemType = 'movie' | 'series' | 'variety' | 'anime';

export interface CatalogCard {
  id: number;
  title: string;
  item_type: CatalogItemType;
  poster?: string;
  progress?: number;
  source_badge?: string;
  update_badge?: string;
}

export type SourceConfidence = 'high' | 'medium' | 'low' | 'unknown'

export interface SourceBadge {
  label: string
  confidence?: SourceConfidence
  tone?: 'warm' | 'cool' | 'neutral' | 'danger'
}

export interface HeroMetric {
  label: string
  value: string
}

export interface HomeHeroCard extends CatalogCard {
  summary?: string
  primary_badge?: string
}

export type EpisodeAvailabilityState = 'playable' | 'resolving' | 'unavailable'

export interface DetailEpisodeView extends CatalogEpisode {
  availability?: EpisodeAvailabilityState
  source_badge?: string
}

interface CatalogCardInputBase {
  id: number;
  title: string;
  poster?: string;
  progress?: number;
  source_badge?: string;
  sourceBadge?: string;
  update_badge?: string;
  updateBadge?: string;
}

export type CatalogCardInput =
  | (CatalogCardInputBase & { item_type: CatalogItemType; itemType?: CatalogItemType })
  | (CatalogCardInputBase & { item_type?: CatalogItemType; itemType: CatalogItemType });

export interface HomePayload {
  continue_watching: CatalogCard[];
  latest_updates: CatalogCard[];
  featured: CatalogCard[];
  douban_hot: DoubanHot[];
}

export interface HomePayloadInput {
  continue_watching?: CatalogCardInput[];
  continueWatching?: CatalogCardInput[];
  latest_updates?: CatalogCardInput[];
  latestUpdates?: CatalogCardInput[];
  featured?: CatalogCardInput[];
  douban_hot?: DoubanHot[];
  doubanHot?: DoubanHot[];
}

export interface CatalogEpisode {
  id: number;
  episode_label: string;
  play_url: string;
  order_index: number;
}

export interface CatalogEpisodeGroup {
  source_name: string;
  episodes: CatalogEpisode[];
}

export interface UnifiedEpisodeSource {
  sourceKey: string
  sourceName: string
  episode: CatalogEpisode
}

export interface UnifiedEpisode {
  normalizedIndex: number
  displayLabel: string
  sources: UnifiedEpisodeSource[]
}

export interface CatalogDetailItem {
  id: number;
  title: string;
  item_type: CatalogItemType;
  poster?: string;
  summary?: string;
  detail_json?: string;
}

export interface CatalogDetail {
  item: CatalogDetailItem;
  episode_groups: CatalogEpisodeGroup[];
}

export interface PlaybackCandidate {
  url: string;
  label: string;
  kind: 'hls' | 'http' | 'external' | 'embed';
  headers?: Record<string, string>;
}

export type PlayerSource = PlaybackCandidate

export interface ResolvedPlayback {
  status: 'ready' | 'failed' | 'external_required';
  candidates: PlaybackCandidate[];
  errorMessage?: string;
}
