import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/notification_service.dart';
import '../theme/app_theme.dart';
import 'machines_screen.dart';
import 'chat_screen.dart';
import 'sessions_screen.dart';
import 'settings_screen.dart';

/// Main screen with bottom navigation: Machines, Chat, Sessions, Settings.
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _currentIndex = 0;

  final _screens = const [
    MachinesScreen(),
    ChatScreen(),
    SessionsScreen(),
    SettingsScreen(),
  ];

  @override
  Widget build(BuildContext context) {
    final notifService = context.watch<NotificationService>();
    final c = context.vibeColors;

    return Scaffold(
      body: IndexedStack(
        index: _currentIndex,
        children: _screens,
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _currentIndex,
        onDestinationSelected: (idx) => setState(() => _currentIndex = idx),
        backgroundColor: c.bgSecondary,
        indicatorColor: c.accentBlue.withValues(alpha: 0.2),
        destinations: [
          NavigationDestination(
            icon: const Icon(Icons.computer_rounded),
            selectedIcon: Icon(Icons.computer_rounded, color: c.accentBlue),
            label: 'Machines',
          ),
          NavigationDestination(
            icon: const Icon(Icons.chat_bubble_outline_rounded),
            selectedIcon: Icon(Icons.chat_bubble_rounded, color: c.accentBlue),
            label: 'Chat',
          ),
          NavigationDestination(
            icon: Badge(
              isLabelVisible: notifService.unreadCount > 0,
              label: Text('${notifService.unreadCount}'),
              child: const Icon(Icons.history_rounded),
            ),
            selectedIcon: Badge(
              isLabelVisible: notifService.unreadCount > 0,
              label: Text('${notifService.unreadCount}'),
              child: Icon(Icons.history_rounded, color: c.accentBlue),
            ),
            label: 'Sessions',
          ),
          NavigationDestination(
            icon: const Icon(Icons.settings_rounded),
            selectedIcon: Icon(Icons.settings_rounded, color: c.accentBlue),
            label: 'Settings',
          ),
        ],
      ),
    );
  }
}
