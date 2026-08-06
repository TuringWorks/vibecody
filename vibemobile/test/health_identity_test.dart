import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:vibecody_mobile/services/api_client.dart';

/// Mobile races every reachable transport (mDNS LAN → Tailscale → ngrok →
/// phone-relay) and adopts the first URL whose `/health` answers. If that check
/// is just "status 200", a captive portal, a proxy, or an unrelated service can
/// win the race and become "the daemon" — after which every request fails in a
/// way that looks like a daemon bug rather than a wrong endpoint.
void main() {
  group('isVibeCliHealthBody', () {
    test('accepts a current daemon that identifies itself', () {
      final body = jsonEncode({
        'status': 'ok',
        'service': 'vibecli',
        'version': '0.5.7',
      });
      expect(isVibeCliHealthBody(body), isTrue);
    });

    test('accepts a pre-`service` daemon via its legacy body shape', () {
      // Older daemons predate the field; mobile talks to a desktop that may
      // not have been upgraded yet, so this must keep working.
      final body = jsonEncode({'status': 'ok', 'version': '0.3.3'});
      expect(isVibeCliHealthBody(body), isTrue);
    });

    test('rejects a different service answering on the same port', () {
      final body = jsonEncode({'status': 'ok', 'service': 'some-other-app'});
      expect(isVibeCliHealthBody(body), isFalse);
    });

    test('rejects a captive portal / proxy returning HTML', () {
      expect(isVibeCliHealthBody('<html><body>Sign in</body></html>'), isFalse);
    });

    test('rejects an empty body', () {
      expect(isVibeCliHealthBody(''), isFalse);
    });

    test('rejects JSON that is not an object', () {
      expect(isVibeCliHealthBody('[1,2,3]'), isFalse);
      expect(isVibeCliHealthBody('"ok"'), isFalse);
    });

    test('rejects an ok-looking body with no version', () {
      // `{"status":"ok"}` is a plausible reply from all sorts of services, so
      // the legacy path deliberately also requires a version string.
      expect(isVibeCliHealthBody(jsonEncode({'status': 'ok'})), isFalse);
    });
  });
}
