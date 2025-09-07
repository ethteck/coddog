// API configuration
// In development, defaults to localhost:3000
// In production, uses the /api proxy path set up in nginx
declare const process: {
  env: {
    API_BASE_URL: string;
  };
};

export const API_BASE_URL = process.env.API_BASE_URL || 'http://localhost:3000';
