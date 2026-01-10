
interface CircularProgressProps {
    value: number;
    color: string;
    label: string;
    sublabel?: string;
    size?: number;
    strokeWidth?: number;
    customValueText?: React.ReactNode;
}

export const CircularProgress = ({ value, color, label, sublabel, size = 120, strokeWidth = 10, customValueText }: CircularProgressProps) => {
    const radius = (size - strokeWidth) / 2;
    const circumference = radius * 2 * Math.PI;
    const offset = circumference - (value / 100) * circumference;

    return (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ position: 'relative', width: size, height: size }}>
                {/* Background Circle */}
                <svg width={size} height={size} style={{ transform: 'rotate(-90deg)' }}>
                    <circle
                        stroke="rgba(255, 255, 255, 0.1)"
                        strokeWidth={strokeWidth}
                        fill="transparent"
                        r={radius}
                        cx={size / 2}
                        cy={size / 2}
                    />
                    {/* Progress Circle */}
                    <circle
                        stroke={color}
                        strokeWidth={strokeWidth}
                        fill="transparent"
                        r={radius}
                        cx={size / 2}
                        cy={size / 2}
                        strokeDasharray={circumference}
                        strokeDashoffset={offset}
                        strokeLinecap="round"
                        style={{ transition: 'stroke-dashoffset 0.5s ease' }}
                    />
                </svg>
                {/* Center Text */}
                <div style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: '100%',
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    pointerEvents: 'none',
                    padding: '0 10px',
                    boxSizing: 'border-box'
                }}>
                    <div style={{
                        fontSize: customValueText ? '0.9rem' : '1.5rem',
                        fontWeight: 700,
                        color: 'var(--text-primary)',
                        textAlign: 'center',
                        lineHeight: '1.2'
                    }}>
                        {customValueText || `${Math.round(value)}%`}
                    </div>
                </div>
            </div>
            <div style={{ marginTop: '0.5rem', textAlign: 'center' }}>
                <div style={{ fontSize: '1rem', color: 'var(--text-primary)', fontWeight: 600 }}>{label}</div>
                {sublabel && <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginTop: '0.1rem' }}>{sublabel}</div>}
            </div>
        </div>
    );
};
