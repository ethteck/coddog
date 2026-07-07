import { Slider } from '@base-ui/react/slider';
import React from 'react';
import styles from './Slider.module.css';

interface SliderProps {
  min: number;
  max: number;
  defaultValue: number;
  onChange?: (value: number) => void;
}

export default function CDSlider({
  min,
  max,
  defaultValue,
  onChange,
}: SliderProps) {
  const [value, setValue] = React.useState(defaultValue);

  return (
    <Slider.Root
      min={min}
      max={max}
      value={value}
      onValueChange={setValue}
      onValueCommitted={onChange}
      className={styles.Root}
    >
      <Slider.Control className={styles.Control}>
        <Slider.Track className={styles.Track}>
          <Slider.Indicator className={styles.Indicator} />
          <Slider.Thumb className={styles.Thumb} />
        </Slider.Track>
      </Slider.Control>
      <Slider.Value />
    </Slider.Root>
  );
}
