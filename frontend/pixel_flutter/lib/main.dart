import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/InputBlocs.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';

import './components//History/HistoryLimit.dart';
import './Views/ProfilePage.dart';

import './Views/TopicsPage.dart';
import './Views/AutenticationPage.dart';

import 'package:pixel_flutter/blocs/MyBlocDelegate.dart';

void main() {
  BlocSupervisor().delegate = MyBlocDelegate();
  runApp(PixelShare());
}

class PixelShare extends StatelessWidget {
  final inputBloc = InputBloc();

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
        bloc: inputBloc,
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

class HomePage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final UserBloc userBloc =
        UserBloc(inputBloc: BlocProvider.of<InputBloc>(context));
    return BlocBuilder(
        bloc: userBloc,
        builder: (BuildContext context, UserState state) {
          if (state is UserNone) {
            return AuthenticationPage();
          } else {
            return TopicsPage();
          }
        });
  }
}
