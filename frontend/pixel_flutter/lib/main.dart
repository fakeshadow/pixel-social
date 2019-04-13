import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import 'package:pixel_flutter/blocs/MyBlocDelegate.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/InputBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';

import './components//History/HistoryLimit.dart';
import './Views/ProfilePage.dart';
import './Views/HomePage.dart';


void main() {
  BlocSupervisor().delegate = MyBlocDelegate();
  runApp(PixelShare());
}

class PixelShare extends StatelessWidget {
  final InputBloc _inputBloc = InputBloc();

  @override
  Widget build(BuildContext context) {
    return BlocProviderTree(
        blocProviders: [
          BlocProvider<InputBloc>(bloc: _inputBloc),
          BlocProvider<UserBloc>(bloc: UserBloc(inputBloc: _inputBloc))
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
}
