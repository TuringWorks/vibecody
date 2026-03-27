import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'services/api_client.dart';
import 'services/auth_service.dart';
import 'services/notification_service.dart';
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
          update: (_, auth, __) => ApiClient(auth: auth),
        ),
        ChangeNotifierProvider(create: (_) => NotificationService()),
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
