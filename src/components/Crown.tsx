interface Props {
  size?: number;
  className?: string;
}

export function Crown({ size = 12, className = "" }: Props) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      className={className}
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M3 8l4 4 5-7 5 7 4-4-1.5 11h-15z" />
      <circle cx="3" cy="7" r="1.6" />
      <circle cx="21" cy="7" r="1.6" />
      <circle cx="12" cy="3.5" r="1.6" />
    </svg>
  );
}
