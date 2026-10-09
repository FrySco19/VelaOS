#pragma once
#include <KQuickConfigModule>
#include <QVariantMap>

// Simple macOS-style appearance page on top of theme's `vela-theme` CLI.
class VelaAspettoKcm : public KQuickConfigModule
{
    Q_OBJECT
    Q_PROPERTY(QVariantMap state READ state NOTIFY stateChanged)
    Q_PROPERTY(bool available READ available CONSTANT)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString error READ error NOTIFY errorChanged)
public:
    VelaAspettoKcm(QObject *parent, const KPluginMetaData &data);
    void load() override;

    QVariantMap state() const { return m_state; }
    bool available() const;
    bool busy() const { return m_running > 0; }
    QString error() const { return m_error; }

    Q_INVOKABLE void setMode(const QString &mode);
    Q_INVOKABLE void setAccent(const QString &accent);
    Q_INVOKABLE void setEffects(const QString &effects);
    Q_INVOKABLE void setMotion(const QString &motion);
    Q_INVOKABLE void setHighContrast(bool on);
    Q_INVOKABLE void setText(const QString &size);
    Q_INVOKABLE bool hasDesktopSettings() const;
    Q_INVOKABLE void openDesktopSettings();
    Q_INVOKABLE void setSchedule(const QString &day, const QString &night);

Q_SIGNALS:
    void stateChanged();
    void busyChanged();
    void errorChanged();

private:
    void run(const QStringList &args, const QString &key, const QVariant &optimistic);
    void refresh();
    QVariantMap m_state;
    int m_running = 0;
    QString m_error;
};
