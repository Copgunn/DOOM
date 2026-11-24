# Quirks in f_finale.c

[Back to the register index](../QUIRKS.md). The quirk register's rows for `f_finale.c`, in line order; how to read a row, the status words and the sources are explained on the index page.

## Rows (1)

| Quirk | C file and line | What a player sees or hears | Status | Small C change removes it? | Sources |
| --- | --- | --- | --- | --- | --- |
| <a id="Q-f_finale-207"></a>**Q-f_finale-207** F_Ticker never ends the Doom 1 finale: after the text the art screen stays and nothing leaves it, so the only way out is the menu. | f_finale.c:207-249 | the art screen stays until the player quits | refused (M6a session: the victory tic is the typed refusal `NotYet(Victory)`; the finale is not built yet; [COMPATIBILITY](../COMPATIBILITY.md#episode-1-as-one-session-m6a)) | unknown: a design question, after the owner's hand test | [M6A](../M6A_REFERENCE_INVESTIGATION.md) §1.8, §5 U8 (11) |
