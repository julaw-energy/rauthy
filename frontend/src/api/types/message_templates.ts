export interface TemplateFieldInfo {
    name: string;
    label: string;
    required: boolean;
    multiline: boolean;
}

export interface TemplatePlaceholderInfo {
    name: string;
    description: string;
    /// the text must contain it
    required: boolean;
}

export interface TemplateInfo {
    id: string;
    title: string;
    description: string;
    /// 'email' (styled mail) | 'message' (plain text for any channel)
    kind: 'email' | 'message';
    fields: TemplateFieldInfo[];
    placeholders: TemplatePlaceholderInfo[];
    /// 'default' is the fallback for every language without its own text
    languages: string[];
    /// the languages changed from the built-in text
    customized: string[];
}

export interface TemplateDetail {
    id: string;
    lang: string;
    values: Record<string, string>;
    defaults: Record<string, string>;
    customized: boolean;
}

export interface TemplateSaveRequest {
    lang: string;
    values: Record<string, string>;
}

export interface TemplateTestRequest {
    lang: string;
    values: Record<string, string>;
    /// for 'message' templates, defaults to 'email'
    channel?: string;
}

export interface TemplatePreviewResponse {
    subject: string;
    text: string;
    html?: string;
    warnings: string[];
}

export interface TemplateTestResponse {
    message: string;
    warnings: string[];
}
