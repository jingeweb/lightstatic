export interface LightStaticOptions {
  root: string;
  noColor?: boolean;
  noAccessLog?: boolean;
  logDir?: string;
  storeInMemory?: boolean;
  cacheForeverRegexp?: string;
  middleware?: string;
  baseHref?: string;
  html5?: boolean;
  index?: string;
  gzip?: boolean;
  open?: boolean;
  bind: string;
  port: number;
  delay: number;
}
