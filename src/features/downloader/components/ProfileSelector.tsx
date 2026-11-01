import { SelectField } from "../../../components/SelectField";

interface ProfileSelectorProps {
    value: string;
    onChange: (value: string) => void;
}

const profiles = [
    { value: "best", label: "Mejor calidad" },
    { value: "davinci-linux", label: "DaVinci Resolve Linux" },
    { value: "youtube", label: "Para YouTube" },
    { value: "small", label: "Menor peso" },
];

export function ProfileSelector({ value, onChange }: ProfileSelectorProps) {
    return (
        <SelectField id="video-profile" label="Perfil" value={value} options={profiles} onChange={onChange} />
    );
}
