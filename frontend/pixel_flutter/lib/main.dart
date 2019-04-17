import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter/blocs/MyBlocDelegate.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';

import './components//History/HistoryLimit.dart';
import './Views/ProfilePage.dart';
import './Views/CategoryPage.dart';
import './Views/CategoryPage.dart';

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
  ErrorBloc errorBloc;

  @override
  void initState() {
    userBloc = UserBloc();
    errorBloc = ErrorBloc();
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocProviderTree(
        blocProviders: [
          BlocProvider<ErrorBloc>(bloc: errorBloc),
          BlocProvider<UserBloc>(bloc: userBloc)
        ],
        child: MaterialApp(
            routes: {
              '/profile': (context) => ProfilePage(),
              '/community': (context) => CategoryPage(),
            },
            theme: ThemeData(
                brightness: Brightness.light,
                primarySwatch: Colors.blue,
                accentColor: Colors.deepPurple),
            navigatorObservers: [HistoryLimit(10)],
            home: CategoryPage()));
  }

  @override
   void dispose() {
    userBloc.dispose();
    super.dispose();
  }
}
