import Frog from '../assets/frog.svg?react';

import styles from './DecompmeLogo.module.css';

export default function DecompmeLogo() {
  return (
    <div className={styles.logo}>
      <Frog className={styles.logoIcon} />
      <span>decomp.me</span>
    </div>
  );
}
