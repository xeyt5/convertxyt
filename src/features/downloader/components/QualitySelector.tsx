interface QuealitySelectorProps{
    value: string;
    onChange: (value:string) => void;
}

const qualities=[
    {
        value: "best",
        label: "mejor calidad disponible"
    },
    {
        value: "2160",
        label: "4k"
    },
    {
        value:"1440",
        label:"2k",
    },
    {
        value:"1080",
        label:"Full HD (1080p)",
    },
    {
        value:"720",
        label:"HD (720p)",
    }
];

export function QuealitySelector({
    value,
    onChange
}: QuealitySelectorProps){
    return(
        <div className="form-group">
            <label htmlFor="video-quality">Calidad</label>
            <select id="video-quality" value={value} onChange={(event) => onChange(event.target.value)}>
                {qualities.map((quality) => (
                    <option key={quality.value} value={quality.value}>
                        {quality.label}
                    </option>
                ))}
            </select>
        </div>
    );

}