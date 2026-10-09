"""Qt stylesheet for the AI terminal built from Vela tokens."""


def qss(c: dict, dark: bool) -> str:
    r = c.get("radius", 12)
    chip_safe_bg = "rgba(92,191,138,0.18)" if dark else "rgba(46,139,87,0.12)"
    chip_change_bg = "rgba(224,178,92,0.20)" if dark else "rgba(183,121,31,0.13)"
    chip_danger_bg = "rgba(230,106,106,0.22)" if dark else "rgba(201,58,58,0.12)"
    return f"""
QMainWindow, QWidget#root {{ background: {c['window']}; color: {c['text']}; }}
QWidget {{ font-family: "{c['font']}"; font-size: 10pt; color: {c['text']}; }}
QWidget#panel {{ background: {c['window']}; border-left: 1px solid {c['separator']}; }}
QWidget#feedInner {{ background: transparent; }}
QScrollArea {{ border: none; background: transparent; }}
QLabel#panelTitle {{ font-size: 11pt; font-weight: 600; }}
QLabel#pill {{ border-radius: 10px; padding: 2px 10px; background: {c['viewAlt']}; color: {c['textSecondary']}; border: 1px solid {c['border']}; font-size: 9pt; }}
QLabel#pill[ok="true"] {{ color: {c['positive']}; }}
QLabel#pill[ok="false"] {{ color: {c['neutral']}; }}
QFrame#card, QFrame#info, QFrame#plan, QFrame#output, QFrame#thinking {{
    background: {c['view']}; border: 1px solid {c['border']}; border-radius: {r}px; }}
QFrame#info {{ border-color: {c['neutral']}; }}
QFrame#welcome {{ background: transparent; border: none; }}
QFrame#cmdItem {{ background: {c['viewAlt']}; border: 1px solid {c['separator']}; border-radius: 8px; }}
QLabel#cardTitle {{ font-weight: 600; font-size: 10.5pt; }}
QLabel#welcomeTitle {{ font-size: 15pt; font-weight: 600; }}
QLabel#muted, QLabel#hint {{ color: {c['textSecondary']}; }}
QLabel#hint {{ font-size: 8.5pt; }}
QLabel#note {{ color: {c['neutral']}; }}
QLabel#reasons {{ color: {c['textSecondary']}; font-size: 9pt; }}
QLabel#reasons[level="danger"] {{ color: {c['negative']}; font-weight: 600; }}
QLabel#inlineWarn {{ color: {c['negative']}; }}
QLabel#finalMsg {{ font-weight: 600; }}
QLabel#userBubble {{ background: {c['accent']}; color: {c['accentText']}; border-radius: {r}px; padding: 8px 12px; }}
QLabel#cmdText {{ color: {c['text']}; }}
QLabel#chipSafe, QLabel#chipChange, QLabel#chipDanger {{ border-radius: 9px; padding: 1px 9px; font-size: 8.5pt; font-weight: 600; }}
QLabel#chipSafe {{ background: {chip_safe_bg}; color: {c['positive']}; }}
QLabel#chipChange {{ background: {chip_change_bg}; color: {c['neutral']}; }}
QLabel#chipDanger {{ background: {chip_danger_bg}; color: {c['negative']}; }}
QLabel#stOk {{ color: {c['positive']}; font-weight: 700; }}
QLabel#stFail {{ color: {c['negative']}; font-weight: 700; }}
QLabel#stRunning {{ color: {c['accent']}; }}
QPushButton {{ background: {c.get('button', c['view'])}; border: 1px solid {c['border']}; border-radius: 8px; padding: 5px 12px; }}
QPushButton:hover {{ border-color: {c['accent']}; }}
QPushButton:disabled {{ color: {c['textDisabled'] if 'textDisabled' in c else c['textSecondary']}; background: {c['viewAlt']}; }}
QPushButton#primary {{ background: {c['accent']}; color: {c['accentText']}; border: none; font-weight: 600; }}
QPushButton#primary:hover {{ background: {c['accentHover']}; }}
QPushButton#primary:disabled, QPushButton#dangerButton:disabled {{ background: {c['border']}; color: {c['textSecondary']}; }}
QPushButton#dangerButton, QPushButton#stopButton {{ background: {c['negative']}; color: white; border: none; font-weight: 600; }}
QPushButton#send {{ background: {c['accent']}; color: {c['accentText']}; border: none; border-radius: 16px; min-width: 32px; min-height: 32px; max-width: 32px; max-height: 32px; font-size: 13pt; padding: 0; }}
QPushButton#send:disabled {{ background: {c['border']}; }}
QToolButton {{ border: none; border-radius: 8px; padding: 4px 8px; background: transparent; }}
QToolButton:hover {{ background: {c['viewAlt']}; }}
QToolButton:checked {{ background: {c['viewAlt']}; color: {c['accent']}; }}
QToolButton#linkButton {{ color: {c['link']}; padding: 0; }}
QPlainTextEdit#preview, QPlainTextEdit#outputBody {{ background: {c.get('field', c['view'])}; border: 1px solid {c['separator']}; border-radius: 8px; padding: 4px; }}
QFrame#composer {{ background: {c.get('field', c['view'])}; border: 1px solid {c['border']}; border-radius: {r}px; }}
QFrame#composer[focus="true"] {{ border-color: {c['accent']}; }}
QPlainTextEdit#composerEdit {{ background: transparent; border: none; }}
QLineEdit {{ background: {c.get('field', c['view'])}; border: 1px solid {c['border']}; border-radius: 8px; padding: 5px 8px; }}
QLineEdit:focus {{ border-color: {c['accent']}; }}
QCheckBox::indicator {{ width: 16px; height: 16px; }}
QTabWidget::pane {{ border: none; }}
QTabBar::tab {{ background: transparent; padding: 6px 14px; margin: 4px 2px; border-radius: 8px; color: {c['textSecondary']}; }}
QTabBar::tab:selected {{ background: {c['view']}; color: {c['text']}; border: 1px solid {c['border']}; }}
QListWidget {{ background: {c['view']}; border: 1px solid {c['border']}; border-radius: 8px; }}
QListWidget::item {{ padding: 8px; border-bottom: 1px solid {c['separator']}; }}
QListWidget::item:selected {{ background: {c['viewAlt']}; color: {c['text']}; }}
QSplitter::handle {{ background: {c['separator']}; width: 1px; }}
QScrollBar:vertical {{ background: transparent; width: 10px; margin: 2px; }}
QScrollBar::handle:vertical {{ background: {c['border']}; border-radius: 4px; min-height: 30px; }}
QScrollBar::add-line, QScrollBar::sub-line {{ height: 0; width: 0; }}
QScrollBar:horizontal {{ background: transparent; height: 10px; margin: 2px; }}
QScrollBar::handle:horizontal {{ background: {c['border']}; border-radius: 4px; min-width: 30px; }}
QMenu {{ background: {c['view']}; border: 1px solid {c['border']}; border-radius: 8px; padding: 4px; }}
QMenu::item {{ padding: 6px 18px; border-radius: 6px; }}
QMenu::item:selected {{ background: {c['accent']}; color: {c['accentText']}; }}
QDialog {{ background: {c['window']}; }}
"""
