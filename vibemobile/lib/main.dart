import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/api_client.dart';
import 'services/auth_service.dart';
import 'services/handoff_service.dart';
import 'services/notification_service.dart';
import 'services/tainted_service.dart';
import 'screens/home_screen.dart';
import 'screens/onboarding_screen.dart';
import 'theme/app_theme.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final authService = AuthService();
  await authService.init();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: authService),
        ProxyProvider<AuthService, ApiClient>(
          update: (_, auth, _) => ApiClient(auth: auth),
        ),
        ChangeNotifierProxyProvider<AuthService, HandoffService>(
          create: (ctx) => HandoffService(ctx.read<AuthService>()),
          update: (_, auth, previous) => previous ?? HandoffService(auth),
        ),
        ChangeNotifierProvider(create: (_) => NotificationService()),
        // DREAD #1 Slice G part 3 — keep a TaintedService subscribed
        // to the first paired machine. The proxy re-configures
        // whenever AuthService machines change, so pair / unpair
        // events transparently swap the SSE target.
        ChangeNotifierProxyProvider2<AuthService, ApiClient, TaintedService>(
          create: (ctx) => TaintedService(api: ctx.read<ApiClient>()),
          update: (_, auth, api, previous) {
            final svc = previous ?? TaintedService(api: api);
            if (auth.machines.isEmpty) {
              svc.configure(null, null);
            } else {
              final m = auth.machines.first;
              svc.configure(m.baseUrl, m.token);
            }
            return svc;
          },
        ),
      ],
      child: const VibeCodyMobileApp(),
    ),
  );
}

class VibeCodyMobileApp extends StatelessWidget {
  const VibeCodyMobileApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'VibeCody',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light,
      darkTheme: AppTheme.dark,
      themeMode: ThemeMode.system,
      home: Consumer<AuthService>(
        builder: (context, auth, _) {
          if (!auth.isInitialized) {
            return const Scaffold(
              body: Center(child: CircularProgressIndicator()),
            );
          }
          return auth.hasMachines ? const HomeScreen() : const OnboardingScreen();
        },
      ),
    );
  }
}
