/// <reference types="vite/client" />

declare module '*.json?url' {
    const value: string;
    export default value;
}

declare module '*.mp3' {
    const value: string;
    export default value;
}
