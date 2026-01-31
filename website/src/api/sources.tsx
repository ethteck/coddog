import { API_BASE_URL } from './config';

export type SourceMetadata = {
  slug: string;
  name: string;
};

export const fetchSourceMetadata = async (
  source_slug: string,
): Promise<SourceMetadata> => {
  const res = await fetch(`${API_BASE_URL}/sources/${source_slug}`);
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};
