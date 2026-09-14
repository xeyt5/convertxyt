interface UrlInputProps{
    value: String;
    onChange: (value: String) => void;
}

export function UrlInput({value, onChange}: UrlInputProps){
    return (
        <div className="form-group">
            <label htmlFor="video-url">URL</label>

            <input id="video-url"
            type="url"
            value={value}
            onChange={(event) => onChange(event.target.value)}
            placeholder="inserta la url"
            />
        </div>
    );
    
}