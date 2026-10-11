/**
 * Copy text to the clipboard, with a fallback for insecure origins.
 *
 * `navigator.clipboard` only exists in a secure context — HTTPS, or localhost.
 * The admin interface is served over plain HTTP on a named host in local
 * development (http://admin.rdatacore.docker), where the whole API is
 * `undefined`. Reading `.writeText` off it throws synchronously, before any
 * promise exists, so a `.catch()` on the chain never runs and the failure is
 * silent: the button appears to do nothing.
 *
 * The fallback is the deprecated `document.execCommand('copy')`, which still
 * works everywhere that matters and has no secure-context requirement. It
 * needs a real selection, hence the off-screen textarea.
 */
export async function copyText(text: string): Promise<boolean> {
    // TypeScript's DOM lib types navigator.clipboard as always present. It is
    // not: outside a secure context the property does not exist at all, which
    // is the whole reason this helper exists. The cast makes the runtime
    // reality visible to the type checker rather than asserting it away.
    const clipboard = navigator.clipboard as Clipboard | undefined

    if (clipboard?.writeText) {
        try {
            await clipboard.writeText(text)

            return true
        } catch {
            // Permission denied or the document is not focused; fall through
            // rather than give up, since execCommand often still succeeds.
        }
    }

    return copyViaExecCommand(text)
}

function copyViaExecCommand(text: string): boolean {
    const textarea = document.createElement('textarea')
    textarea.value = text
    // Off-screen rather than hidden: a display:none element cannot be
    // selected, and `readonly` stops a mobile keyboard appearing.
    textarea.setAttribute('readonly', '')
    textarea.style.position = 'fixed'
    textarea.style.top = '-9999px'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)

    try {
        textarea.select()
        textarea.setSelectionRange(0, text.length)

        return document.execCommand('copy')
    } catch {
        return false
    } finally {
        document.body.removeChild(textarea)
    }
}
