import { SelectField } from "../../../components/SelectField";

interface QualitySelectorProps {
    value: string;
    onChange: (value: string) => void;
}

const qualities = [
    { value: "best", label: "Mejor disponible" },
    { value: "2160", label: "4K" },
    { value: "1440", label: "2K" },
    { value: "1080", label: "Full HD (1080p)" },
    { value: "720", label: "HD (720p)" },
];

export function QualitySelector({ value, onChange }: QualitySelectorProps) {
    return (
        <SelectField id="video-quality" label="Calidad" value={value} options={qualities} onChange={onChange} />
    );
}
