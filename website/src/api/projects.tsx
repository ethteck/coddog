import { API_BASE_URL } from './config';

export type Project = {
  id: number;
  name: string;
  platform: number;
  repo?: string;
};

export const fetchProjects = async (): Promise<Array<Project>> => {
  const res = await fetch(`${API_BASE_URL}/projects`);
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};
