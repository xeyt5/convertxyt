interface ProfileSelectorProps{
    value: string;
    onChange: (value: string) => void;
}

const profiles = [
    {
        value: "best",
        label: "mejor calidad",
    },
    {
        value: "davinci-linux",
        label: "Davinci resolve Linux",
    },
    {
        value: "youtube",
        label: "Para youtube",
    },
    {
        value: "small",
        label: "Menor peso",
    },
];

export function ProfileSelector({
    value,
    onChange,
}: ProfileSelectorProps){
    return(
    <div className="form-group">
        <label htmlFor="video-profile">Perfil</label>

        <select id="video-profile" value={value} onChange={(event) => onChange(event.target.value)}>
        {profiles.map((profile) => (
            <option key={profile.value} value={profile.value}>
                {profile.label}
            </option>
        ))}
        </select>
    </div>
    );
}