VPont Pebble Index 01 → jtx Board (Rust)
Petit service Rust qui reçoit le webhook de l'appli Pebble (Index 01), convertit la transcription en entrée iCalendar (VJOURNAL ou VTODO) et l'écrit dans une collection CalDAV Nextcloud, synchronisée vers jtx Board par DAVx⁵.
Liens de référence
• Guide de démarrage Index 01 : https://help.repebble.com/en/articles/15434751-index-01-getting-started-guide
• Fonctions avancées (MCP et webhooks) : https://help.repebble.com/en/articles/15724406-index-advanced-features-mcp-webhook
• Doc SDK Index 01 (webhooks v1, signature, exemples) : https://github.com/coredevices/sdk-docs/pull/26
• Code de l'appli mobile (source de vérité du format) : https://github.com/coredevices/mobileapp
• Page produit : https://repebble.com/index
• Exemples communautaires de récepteurs :
    ◦ https://github.com/maimon495/pebble-index-webhook (Cloudflare Worker)
    ◦ https://github.com/lab909/pebble-index-01-webhook-hub (PHP)
    ◦ https://github.com/rbartoli/pebble-index
• CalDAV : RFC 4791 — iCalendar : RFC 5545
• jtx Board : https://jtx.techbee.at
Architecture
1. Index 01 : appui sur le bouton, enregistrement audio.
2. Appli Pebble : transcription (locale ou cloud selon réglage), puis POST du webhook.
3. Service Rust (cluster Kubernetes) : vérifie la signature, classe le contenu, génère l'ICS.
4. Nextcloud : PUT CalDAV dans la collection dédiée.
5. DAVx⁵ : synchronisation vers jtx Board.
Spécification du webhook entrant
À confirmer dans la doc SDK et dans « Recent runs » de l'appli (envoyer un événement de test).
• Méthode : POST, corps multipart/form-data.
• Champs attendus : transcription texte, horodatage d'enregistrement, audio optionnel.
• En-têtes : signature HMAC avec secret partagé, horodatage, identifiant de livraison.
• Sécurité : vérifier la signature sur les octets exacts décrits dans la doc, rejeter les horodatages hors fenêtre, ignorer les identifiants déjà vus (anti-rejeu).
• Événement de test : répondre 200 sans rien écrire.
• Réponses : 200 succès, 401 signature invalide, 400 corps invalide, 502 si Nextcloud échoue.
Classification
Option A, par geste : un chemin d'URL par geste configuré dans l'appli.
• /hook/note → VJOURNAL sans date (note)
• /hook/journal → VJOURNAL avec DTSTART (journal)
• /hook/todo → VTODO
Option B, par mot-clé en début de phrase (FR et EN) :
• « tâche », « à faire », « todo » → VTODO
• « journal » → journal
• sinon → note
Le mot-clé est retiré du texte. Le résumé (SUMMARY) prend la première phrase ou les 60 premiers caractères, la description le texte complet.
Génération iCalendar
Note :
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//pebble-jtx//FR
BEGIN:VJOURNAL
UID:<uuid>
DTSTAMP:<maintenant UTC>
SUMMARY:<résumé>
DESCRIPTION:<texte>
CATEGORIES:Pebble
END:VJOURNAL
END:VCALENDAR
Journal : identique avec DTSTART à l'heure d'enregistrement. Tâche : VTODO avec STATUS:NEEDS-ACTION.
Penser à l'échappement RFC 5545 (virgules, points-virgules, retours ligne) et au pliage des lignes à 75 octets.
Écriture CalDAV
• URL : https://<nextcloud>/remote.php/dav/calendars/<user>/<collection>/<uid>.ics
• PUT avec Content-Type: text/calendar; charset=utf-8 et If-None-Match: *
• Authentification : Basic avec un mot de passe d'application Nextcloud dédié.
• Succès : 201 Created.
Stack Rust suggérée
• axum + tokio : serveur HTTP et multipart
• hmac + sha2 + subtle : vérification de signature en temps constant
• reqwest (rustls) : client CalDAV
• uuid, chrono : identifiants et dates
• icalendar (optionnel) : génération ICS
• tracing : logs
• serde + envy : configuration par variables d'environnement
Configuration
• PEBBLE_WEBHOOK_SECRET
• CALDAV_URL (URL de la collection)
• CALDAV_USER, CALDAV_APP_PASSWORD
• CLASSIFY_MODE = path ou keyword
• TZ (le webhook n'envoie pas le fuseau du téléphone)
Déploiement
• Image multi-stage (rust puis distroless ou scratch avec musl).
• Deployment + Service + Ingress TLS, secrets dans un Secret Kubernetes.
• Endpoint /healthz pour les probes.
• Ne pas exposer d'autre route que le webhook.
Étapes de test
1. Configurer le webhook dans l'appli Pebble et envoyer l'événement de test.
2. Noter le format réel dans « Recent runs » et ajuster le parsing.
3. Enregistrer une note, vérifier le fichier .ics sur Nextcloud.
4. Synchroniser DAVx⁵ et vérifier dans jtx Board.
Évolutions possibles
• Serveur MCP CalDAV pour laisser l'agent choisir le type.
• Archivage de l'audio dans Nextcloud et lien en pièce jointe (ATTACH).
• Dates d'échéance extraites du texte pour les tâches.
