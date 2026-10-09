#pragma once
#include <KQuickConfigModule>
#include <QJsonObject>

// Common editing model: effective config = system defaults merged with the user file.
// QML reads with value(path) (bind on `revision` to refresh) and writes with setValue(path, v).
class VelaKcmBase : public KQuickConfigModule
{
    Q_OBJECT
    Q_PROPERTY(int revision READ revision NOTIFY revisionChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)
public:
    VelaKcmBase(QObject *parent, const KPluginMetaData &data);

    Q_INVOKABLE QVariant value(const QString &path) const;
    Q_INVOKABLE QVariant systemValue(const QString &path) const;
    Q_INVOKABLE void setValue(const QString &path, const QVariant &v);

    int revision() const { return m_revision; }
    QString lastError() const { return m_error; }

    void load() override;
    void save() override;
    void defaults() override;

Q_SIGNALS:
    void revisionChanged();
    void lastErrorChanged();

protected:
    // keys under these prefixes are managed by this KCM (defaults() resets only them)
    virtual QStringList ownedPrefixes() const = 0;
    virtual void loadExtra() {}
    virtual bool saveExtra(QString *) { return true; }
    virtual void defaultsExtra() {}
    bool owns(const QString &leaf) const;
    void bump();
    void setError(const QString &e);
    void updateState();
    virtual bool extraDirty() const { return false; }
    virtual bool extraIsDefault() const { return true; }

    QJsonObject m_system;
    QJsonObject m_user;      // as on disk + pending edits
    QJsonObject m_userSaved; // as on disk
    int m_revision = 0;
    QString m_error;
};
