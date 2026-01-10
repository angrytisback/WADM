import React from 'react';

interface SplitCircularProgressProps {
    leftValue: number; 
    rightValue: number; 
    size?: number;
    strokeWidth?: number;
    label: string;
    sublabel?: string;
    customValueText?: React.ReactNode;
}

export const SplitCircularProgress: React.FC<SplitCircularProgressProps> = ({
    leftValue,
    rightValue,
    size = 140,
    strokeWidth = 12,
    label,
    sublabel,
    customValueText,
}) => {
    const radius = (size - strokeWidth) / 2;
    const center = size / 2;
    
    const gap = 10;

    
    
    

    
    const createArc = (startAngle: number, endAngle: number, color: string) => {
        const startRad = (startAngle - 90) * (Math.PI / 180.0);
        const endRad = (endAngle - 90) * (Math.PI / 180.0);

        const x1 = center + radius * Math.cos(startRad);
        const y1 = center + radius * Math.sin(startRad);
        const x2 = center + radius * Math.cos(endRad);
        const y2 = center + radius * Math.sin(endRad);

        
        const largeArcFlag = endAngle - startAngle <= 180 ? "0" : "1";

        return (
            <path
                d={`M ${x1} ${y1} A ${radius} ${radius} 0 ${largeArcFlag} 1 ${x2} ${y2}`}
                fill="none"
                stroke={color}
                strokeWidth={strokeWidth}
                strokeLinecap="round"
            />
        );
    };

    
    
    
    
    
    
    
    

    
    

    
    

    
    
    

    
    
    

    const rightStart = gap;
    const rightEnd = 180 - gap;
    const rightRange = rightEnd - rightStart;
    const rightProgressEnd = rightStart + (Math.min(rightValue, 100) / 100) * rightRange;

    const leftStart = 180 + gap;
    const leftEnd = 360 - gap;
    const leftRange = leftEnd - leftStart;
    const leftProgressEnd = leftStart + (Math.min(leftValue, 100) / 100) * leftRange;

    
    const downloadColor = "#a78bfa"; 
    const uploadColor = "#c4b5fd";   
    const bgOpacity = 0.1;

    return (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ position: 'relative', width: size, height: size }}>
                <svg width={size} height={size} style={{ transform: 'rotate(-90deg)' }}>
                    {/* Right Background (Upload) */}
                    {createArc(rightStart, rightEnd, `rgba(196, 181, 253, ${bgOpacity})`)}
                    {/* Right Progress (Upload) */}
                    {createArc(rightStart, rightProgressEnd, uploadColor)}

                    {/* Left Background (Download) */}
                    {createArc(leftStart, leftEnd, `rgba(167, 139, 250, ${bgOpacity})`)}
                    {/* Left Progress (Download) */}
                    {createArc(leftStart, leftProgressEnd, downloadColor)}
                </svg>

                {/* Center Content */}
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
                        fontSize: '0.9rem',
                        fontWeight: 700,
                        color: 'var(--text-primary)',
                        textAlign: 'center',
                        lineHeight: '1.2'
                    }}>
                        {customValueText}
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
