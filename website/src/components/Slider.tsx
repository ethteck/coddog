import { Slider as BaseSlider } from '@base-ui-components/react/slider';
import styles from './Slider.module.css';

interface SliderProps {
  min: number;
  max: number;
  value: number;
  onChange?: (value: number) => void;
}

export default function Slider({ min, max, value, onChange }: SliderProps) {
  return (
    <BaseSlider.Root
      min={min}
      max={max}
      value={value}
      onValueCommitted={onChange}
      className={styles.Root}
    >
      <BaseSlider.Control className={styles.Control}>
        <BaseSlider.Track className={styles.Track}>
          <BaseSlider.Indicator className={styles.Indicator} />
          <BaseSlider.Thumb className={styles.Thumb} />
        </BaseSlider.Track>
      </BaseSlider.Control>
      <BaseSlider.Value />
    </BaseSlider.Root>
  );
}
