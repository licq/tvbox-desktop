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
  type: 'movie' | 'tv' | 'variety' | 'anime';
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
