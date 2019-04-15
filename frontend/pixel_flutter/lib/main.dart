import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import 'package:pixel_flutter/blocs/MyBlocDelegate.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';

import './components//History/HistoryLimit.dart';
import './Views/ProfilePage.dart';
import './Views/HomePage.dart';


void main() {
  BlocSupervisor().delegate = MyBlocDelegate();
  runApp(PixelShare());
}

class PixelShare extends StatefulWidget {
  @override
  _PixelShareState createState() => _PixelShareState();
}

class _PixelShareState extends State<PixelShare> {
  UserBloc userBloc;

  @override
  void initState() {
    userBloc = UserBloc();
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocProviderTree(
        blocProviders: [
          BlocProvider<UserBloc>(bloc: userBloc)
        ],
        child: MaterialApp(
            routes: {
              '/profile': (context) => ProfilePage(),
              '/community': (context) => PixelShare(),
            },
            theme: ThemeData(
                brightness: Brightness.light,
                primarySwatch: Colors.blue,
                accentColor: Colors.deepPurple),
            navigatorObservers: [HistoryLimit(10)],
            home: HomePage()));
  }

  @override
  void dispose() {
    userBloc.dispose();
    super.dispose();
  }
}
