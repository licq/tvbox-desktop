export interface Subscription {
  id: number;
  name: string;
  url: string;
  enabled: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface LiveChannel {
  id: number;
  subscription_id: number;
  name: string;
  logo?: string;
  url: string;
  category?: string;
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
