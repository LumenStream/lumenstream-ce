import { cn } from "@/lib/utils";

interface SliderProps {
  value: number[];
  onValueChange: (value: number[]) => void;
  min?: number;
  max?: number;
  step?: number;
  className?: string;
}

export function Slider({
  value,
  onValueChange,
  min = 0,
  max = 100,
  step = 1,
  className,
}: SliderProps) {
  const percentage = ((value[0] - min) / (max - min)) * 100;

  return (
    <div className={cn("relative w-full", className)}>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value[0]}
        onChange={(e) => onValueChange([Number(e.target.value)])}
        className="slider-input h-2 w-full cursor-pointer appearance-none rounded-full bg-slate-700"
        style={{
          background: `linear-gradient(to right, #8b5cf6 0%, #8b5cf6 ${percentage}%, #334155 ${percentage}%, #334155 100%)`,
        }}
      />
      <style>{`
        .slider-input::-webkit-slider-thumb {
          -webkit-appearance: none;
          appearance: none;
          width: 20px;
          height: 20px;
          border-radius: 50%;
          background: linear-gradient(135deg, #a78bfa, #8b5cf6);
          cursor: pointer;
          border: 2px solid #1e293b;
          box-shadow: 0 0 10px rgba(139, 92, 246, 0.5);
          transition: transform 0.15s ease, box-shadow 0.15s ease;
        }
        .slider-input::-webkit-slider-thumb:hover {
          transform: scale(1.1);
          box-shadow: 0 0 15px rgba(139, 92, 246, 0.7);
        }
        .slider-input::-moz-range-thumb {
          width: 20px;
          height: 20px;
          border-radius: 50%;
          background: linear-gradient(135deg, #a78bfa, #8b5cf6);
          cursor: pointer;
          border: 2px solid #1e293b;
          box-shadow: 0 0 10px rgba(139, 92, 246, 0.5);
        }
        .slider-input:focus {
          outline: none;
        }
      `}</style>
    </div>
  );
}
