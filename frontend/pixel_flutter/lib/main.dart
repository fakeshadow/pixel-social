import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import './components//History/HistoryLimit.dart';
import './Views/ProfilePage.dart';

import './Views/TopicsPage.dart';

import './blocs/Provider.dart';
import './components/Auth/LoginScreen.dart';

void main() => runApp(RootApp());

class RootApp extends StatefulWidget {
  @override
  RootAppState createState() => RootAppState();
}

class RootAppState extends State<RootApp> {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
        routes: {
          '/profile': (context) => ProfilePage(),
          '/community': (context) => RootApp(),
        },
        theme: ThemeData(
            brightness: Brightness.light,
            primarySwatch: Colors.blue,
            accentColor: Colors.deepPurple),
        navigatorObservers: [HistoryLimit(10)],
        home: Provider(
          child: Scaffold(body: LoginScreen()),
        ));
  }
}
