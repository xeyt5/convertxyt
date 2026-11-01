interface UrlInputProps {
    value: string;
    onChange: (value: string) => void;
}

export function UrlInput({ value, onChange }: UrlInputProps) {
    return (
        <label className="row row-input">
            <span className="row-caption">URL</span>
            <input
                type="url"
                value={value}
                onChange={(event) => onChange(event.target.value)}
                spellCheck={false}
            />
        </label>
    );
}
