interface MascotSvgProps {
  size: number;
}

export function CodeBuddyMascot({ size }: MascotSvgProps) {
  return (
    <img
      src="/icons/codebuddy-logo.png"
      alt="CodeBuddy"
      width={size}
      height={size}
      style={{ objectFit: "contain" }}
    />
  );
}
