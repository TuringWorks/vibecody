---
triggers: ["Firebase", "firebase", "firebase auth", "firebase hosting", "firebase messaging", "firebase analytics", "firebase emulator", "firebase rules"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["firebase"]
category: cloud-firebase
---

# Firebase Platform

When working with Firebase:

1. Initialize a Firebase project with `firebase init` selecting the desired features (Firestore, Auth, Hosting, Functions, Emulators); configure `firebase.json` with hosting rewrites, function regions, and emulator ports, then deploy with `firebase deploy` or `firebase deploy --only functions` for targeted deploys.
2. Use Firestore for document-based data with the modular v9+ SDK: `import { collection, addDoc, query, where, getDocs } from 'firebase/firestore'; const q = query(collection(db, 'posts'), where('published', '==', true)); const snapshot = await getDocs(q);` and enable offline persistence with `enableIndexedDbPersistence(db)`.
3. Implement authentication with `import { getAuth, signInWithPopup, GoogleAuthProvider } from 'firebase/auth'; const auth = getAuth(); const result = await signInWithPopup(auth, new GoogleAuthProvider());` and listen for state changes with `onAuthStateChanged(auth, (user) => { ... })` to gate access across the app.
4. Write Firestore Security Rules in `firestore.rules`: `rules_version = '2'; service cloud.firestore { match /databases/{database}/documents { match /posts/{postId} { allow read: if true; allow write: if request.auth != null && request.auth.uid == resource.data.authorId; } } }` and test with `firebase emulators:exec "npm test"`.
5. Deploy Cloud Functions (2nd gen) with: `import { onRequest } from 'firebase-functions/v2/https'; import { onDocumentCreated } from 'firebase-functions/v2/firestore'; export const api = onRequest({ region: 'us-central1', memory: '256MiB' }, async (req, res) => { res.json({ ok: true }); });` and configure scaling with `minInstances` and `maxInstances`.
6. Set up Firebase Hosting with `firebase.json`: `{ "hosting": { "public": "dist", "rewrites": [{ "source": "/api/**", "function": "api" }, { "source": "**", "destination": "/index.html" }], "headers": [{ "source": "**/*.@(js|css)", "headers": [{ "key": "Cache-Control", "value": "max-age=31536000" }] }] } }` for SPA routing and asset caching.
7. Integrate Cloud Messaging (FCM) for push notifications: `import { getMessaging, getToken, onMessage } from 'firebase/messaging'; const token = await getToken(messaging, { vapidKey: 'YOUR_VAPID_KEY' });` on the client, and send from the server with `import { getMessaging } from 'firebase-admin/messaging'; await getMessaging().send({ token, notification: { title, body } });`.
8. Use Remote Config for feature flags and A/B testing: `import { getRemoteConfig, fetchAndActivate, getValue } from 'firebase/remote-config'; const rc = getRemoteConfig(app); rc.settings.minimumFetchIntervalMillis = 3600000; await fetchAndActivate(rc); const enabled = getValue(rc, 'new_feature').asBoolean();`.
9. Run the Emulator Suite for local development with `firebase emulators:start` which launches Auth, Firestore, Functions, Hosting, and Storage emulators; configure ports in `firebase.json` under `"emulators"` and use `connectAuthEmulator(auth, 'http://localhost:9099')` in client code to target local services.
10. Enable App Check to protect backend resources from abuse: `import { initializeAppCheck, ReCaptchaV3Provider } from 'firebase/app-check'; initializeAppCheck(app, { provider: new ReCaptchaV3Provider('SITE_KEY'), isTokenAutoRefreshEnabled: true });` and enforce in Security Rules with `request.app.token.app_check == true`.
11. Optimize costs by using Firestore composite indexes (defined in `firestore.indexes.json`) to avoid full collection scans, batching writes with `writeBatch(db)` to reduce document write charges, enabling Firestore TTL policies for auto-deletion of temporary data, and using `firebase functions:log` to identify high-invocation functions.
12. Use Firebase Extensions for pre-built backend functionality: `firebase ext:install firebase/firestore-send-email` for transactional emails, `firebase/storage-resize-images` for automatic image processing, and `firebase/firestore-translate-text` for multi-language support, reducing custom function code and maintenance overhead.
