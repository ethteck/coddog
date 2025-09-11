import { Link } from '@tanstack/react-router';
import { isDecompmeScratch, type SymbolMetadata } from '../api/symbols.tsx';
import GBALogo from '../assets/platforms/gba.svg?react';
import GCWiiLogo from '../assets/platforms/gcwii.svg?react';
import N64Logo from '../assets/platforms/n64.svg?react';
import NDSLogo from '../assets/platforms/nds.svg?react';
import PS2Logo from '../assets/platforms/ps2.svg?react';
import PSXLogo from '../assets/platforms/psx.svg?react';
import DecompmeLogo from './DecompmeLogo.tsx';

import styles from './SymbolLabel.module.css';

function getPlatformLogo(symbol: SymbolMetadata) {
  const platform = symbol.platform;

  switch (platform) {
    case 0: // N64
      return <N64Logo className={styles.logo} />;
    case 1: // PSX
      return <PSXLogo className={styles.logo} />;
    case 2: // PS2
      return <PS2Logo className={styles.logo} />;
    case 3: // GC/Wii
      return <GCWiiLogo className={styles.logo} />;
    case 4: // PSP
      return 'PSP';
    case 5: // GBA
      return <GBALogo className={styles.logo} />;
    case 6: // NDS
      return <NDSLogo className={styles.logo} />;
    case 7: // N3DS
      return 'N3DS';
    default:
      return null;
  }
}

export function SymbolLabel({
  symbol,
  link = true,
  className = '',
}: {
  symbol: SymbolMetadata;
  link?: boolean;
  className?: string;
}) {
  const platformLogo = getPlatformLogo(symbol);

  const content = isDecompmeScratch(symbol) ? (
    <>
      {platformLogo && <>{platformLogo} </>}
      <b>{symbol.name}</b> - <DecompmeLogo />/{symbol.source_name}
    </>
  ) : (
    <>
      {platformLogo && <>{platformLogo} </>}
      <b>{symbol.name}</b> - {symbol.project_name}
      {symbol.version_name ? ` (${symbol.version_name})` : ''}
    </>
  );

  return link ? (
    <Link
      to="/symbol/$symbolSlug"
      params={{ symbolSlug: symbol.slug }}
      className={className}
    >
      {content}
    </Link>
  ) : (
    <span className={className}>{content}</span>
  );
}
