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

export interface DoubanHotItem {
  id: number;
  name: string;
  year?: number;
  poster?: string;
  rating?: number;
  rank: number;
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
}

export interface HomePayloadInput {
  continue_watching?: CatalogCardInput[];
  continueWatching?: CatalogCardInput[];
  latest_updates?: CatalogCardInput[];
  latestUpdates?: CatalogCardInput[];
  featured?: CatalogCardInput[];
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

export interface ResolvedPlayback {
  status: 'ready' | 'failed' | 'external_required';
  candidates: PlaybackCandidate[];
  errorMessage?: string;
}
