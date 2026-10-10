#include "kcm_vela_ai.h"
#include <QFile>
#include <QStandardPaths>
#include <QProcess>
#include "velaconfig.h"

#include <KLocalizedString>
#include <KPluginFactory>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkRequest>
#include <QUrl>
#include <QProcess>
#include <QStandardPaths>
#include <QTimer>

K_PLUGIN_CLASS_WITH_JSON(VelaAiKcm, "kcm_vela_ai.json")

VelaAiKcm::VelaAiKcm(QObject *parent, const KPluginMetaData &data)
    : VelaKcmBase(parent, data)
{
    m_nam.setTransferTimeout(15000);
}

QStringList VelaAiKcm::ownedPrefixes() const
{
    return {QStringLiteral("enabled"), QStringLiteral("provider"), QStringLiteral("ollama"), QStringLiteral("openai"),
            QStringLiteral("generation"), QStringLiteral("language"),
            QStringLiteral("assistant.history"), QStringLiteral("assistant.tray"),
            QStringLiteral("permissions.active_window"), QStringLiteral("permissions.selected_text"),
            QStringLiteral("permissions.clipboard"), QStringLiteral("permissions.files"),
            QStringLiteral("permissions.screen"), QStringLiteral("permissions.microphone"),
            QStringLiteral("privacy.allow_cloud")};
}

void VelaAiKcm::loadExtra()
{
    m_apiKey = VelaConfig::readApiKey();
    m_apiKeySaved = m_apiKey;
    Q_EMIT apiKeyChanged();
}

bool VelaAiKcm::saveExtra(QString *err)
{
    if (value(QStringLiteral("openai.base_url")).toString() == QLatin1String("http://127.0.0.1:11435/v1")) {
        QProcess::startDetached(QStringLiteral("systemctl"), {QStringLiteral("--user"),
            value(QStringLiteral("enabled")).toBool() ? QStringLiteral("start") : QStringLiteral("stop"),
            QStringLiteral("vela-local-ai.service")});
    }
    if (m_apiKey == m_apiKeySaved) return true;
    if (!VelaConfig::writeApiKey(m_apiKey, err)) return false;
    m_apiKeySaved = m_apiKey;
    return true;
}

void VelaAiKcm::defaultsExtra() {}

void VelaAiKcm::setApiKey(const QString &k)
{
    if (k == m_apiKey) return;
    m_apiKey = k;
    Q_EMIT apiKeyChanged();
    updateState();
}

bool VelaAiKcm::isCloudUrl(const QString &url) const
{
    const QString host = QUrl(url).host();
    return !(host == QLatin1String("127.0.0.1") || host == QLatin1String("localhost") || host == QLatin1String("::1") || host.isEmpty());
}

void VelaAiKcm::setTest(int state, const QString &msg)
{
    m_testState = state;
    m_testMessage = msg;
    Q_EMIT testChanged();
}

void VelaAiKcm::testConnection()
{
    if (m_testReply) m_testReply->abort();
    const QString provider = value(QStringLiteral("provider")).toString();
    QNetworkRequest req;
    if (provider == QLatin1String("openai")) {
        QString base = value(QStringLiteral("openai.base_url")).toString();
        while (base.endsWith(QLatin1Char('/'))) base.chop(1);
        if (isCloudUrl(base) && !value(QStringLiteral("privacy.allow_cloud")).toBool()) {
            setTest(3, i18nd("kcm_vela_ai", "Sending data to online services is turned off. Turn it on below or in Privacy and memory."));
            return;
        }
        if (!QUrl(base).isValid() || QUrl(base).scheme().isEmpty()) {
            setTest(3, i18nd("kcm_vela_ai", "The server address is not valid."));
            return;
        }
        req.setUrl(QUrl(base + QStringLiteral("/models")));
        if (base == QLatin1String("http://127.0.0.1:11435/v1")) {
            QFile key(QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation) + QStringLiteral("/vela-local-ai.key"));
            if (key.open(QIODevice::ReadOnly)) req.setRawHeader("Authorization", "Bearer " + key.readAll().trimmed());
        } else if (hasApiKey()) req.setRawHeader("Authorization", "Bearer " + m_apiKey.trimmed().toUtf8());
    } else {
        QString base = value(QStringLiteral("ollama.base_url")).toString();
        while (base.endsWith(QLatin1Char('/'))) base.chop(1);
        req.setUrl(QUrl(base + QStringLiteral("/api/tags")));
    }
    setTest(1, i18nd("kcm_vela_ai", "Checking connection…"));
    m_testReply = m_nam.get(req);
    QNetworkReply *reply = m_testReply;
    connect(reply, &QNetworkReply::finished, this, [this, reply, provider]() {
        reply->deleteLater();
        if (reply->error() == QNetworkReply::OperationCanceledError) return;
        const int status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
        if (reply->error() != QNetworkReply::NoError) {
            if (status == 401 || status == 403) {
                setTest(3, i18nd("kcm_vela_ai", "The service rejected the API key. Check that it is correct."));
            } else if (provider != QLatin1String("openai")) {
                setTest(3, i18nd("kcm_vela_ai", "The local model is not running. Make sure Ollama is installed and started."));
            } else {
                setTest(3, i18nd("kcm_vela_ai", "Cannot reach the service. Check the address and the internet connection."));
            }
            return;
        }
        const auto doc = QJsonDocument::fromJson(reply->readAll());
        QStringList names;
        if (provider == QLatin1String("openai")) {
            for (const auto &m : doc.object().value(QStringLiteral("data")).toArray())
                names << m.toObject().value(QStringLiteral("id")).toString();
        } else {
            for (const auto &m : doc.object().value(QStringLiteral("models")).toArray())
                names << m.toObject().value(QStringLiteral("name")).toString();
        }
        names.removeAll(QString());
        names.sort(Qt::CaseInsensitive);
        m_models = names;
        Q_EMIT modelsChanged();
        if (names.isEmpty() && provider != QLatin1String("openai")) {
            setTest(2, i18nd("kcm_vela_ai", "Connected. No model is installed yet: download one below."));
        } else {
            setTest(2, i18ndp("kcm_vela_ai", "Connected. %1 model available.", "Connected. %1 models available.", names.size()));
        }
    });
}

void VelaAiKcm::pullModel(const QString &name)
{
    const QString model = name.trimmed();
    if (model.isEmpty() || m_pulling) return;
    QString base = value(QStringLiteral("ollama.base_url")).toString();
    while (base.endsWith(QLatin1Char('/'))) base.chop(1);
    QNetworkRequest req(QUrl(base + QStringLiteral("/api/pull")));
    req.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    req.setTransferTimeout(0);
    QJsonObject body{{QStringLiteral("model"), model}, {QStringLiteral("stream"), true}};
    m_pulling = true;
    m_pullProgress = 0;
    m_pullStatus = i18nd("kcm_vela_ai", "Downloading %1…", model);
    m_pullBuf.clear();
    Q_EMIT pullChanged();
    m_pullReply = m_nam.post(req, QJsonDocument(body).toJson(QJsonDocument::Compact));
    QNetworkReply *reply = m_pullReply;
    connect(reply, &QNetworkReply::readyRead, this, [this, reply, model]() {
        m_pullBuf += reply->readAll();
        int nl;
        while ((nl = m_pullBuf.indexOf('\n')) >= 0) {
            const QByteArray line = m_pullBuf.left(nl);
            m_pullBuf.remove(0, nl + 1);
            const auto o = QJsonDocument::fromJson(line).object();
            if (o.contains(QStringLiteral("error"))) {
                m_pullStatus = i18nd("kcm_vela_ai", "Download failed: %1", o.value(QStringLiteral("error")).toString());
            } else if (o.contains(QStringLiteral("total")) && o.value(QStringLiteral("total")).toDouble() > 0) {
                m_pullProgress = o.value(QStringLiteral("completed")).toDouble() / o.value(QStringLiteral("total")).toDouble();
                m_pullStatus = i18nd("kcm_vela_ai", "Downloading %1… %2%", model, int(m_pullProgress * 100));
            } else if (o.value(QStringLiteral("status")).toString() == QLatin1String("success")) {
                m_pullProgress = 1;
                m_pullStatus = i18nd("kcm_vela_ai", "%1 is ready.", model);
            }
            Q_EMIT pullChanged();
        }
    });
    connect(reply, &QNetworkReply::finished, this, [this, reply, model]() {
        reply->deleteLater();
        m_pulling = false;
        if (reply->error() == QNetworkReply::OperationCanceledError) {
            m_pullStatus = i18nd("kcm_vela_ai", "Download cancelled.");
        } else if (reply->error() != QNetworkReply::NoError) {
            m_pullStatus = i18nd("kcm_vela_ai", "Download failed. Check the internet connection and that Ollama is running.");
        } else if (m_pullProgress >= 1) {
            setValue(QStringLiteral("ollama.model"), model);
        }
        Q_EMIT pullChanged();
        testConnection();
    });
}

bool VelaAiKcm::canSetupLocal() const
{
    return !QStandardPaths::findExecutable(QStringLiteral("vela-assistant")).isEmpty();
}

void VelaAiKcm::setupLocal()
{
    // The assistant owns the (per-user) Ollama installer; we only start it on an explicit click.
    if (!QProcess::startDetached(QStringLiteral("vela-assistant"), {QStringLiteral("--setup-local")})) {
        setTest(3, i18nd("kcm_vela_ai", "Cannot start the installer of the local model."));
    }
}

void VelaAiKcm::cancelPull()
{
    if (m_pullReply) m_pullReply->abort();
}

#include "kcm_vela_ai.moc"
