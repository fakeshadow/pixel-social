import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:bloc/bloc.dart';

import 'package:pixel_flutter/blocs/InputBlocs.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserEvent.dart';

import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/models/User.dart';

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
  final InputBloc _inputBloc = InputBloc();

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _inputBloc,
        builder: (BuildContext context, InputState state) {
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
        });
  }
}

class HomePage extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: BlocProvider.of<UserBloc>(context),
        builder: (BuildContext context, UserState state) {
          if (state is AppStarted) {
            BlocProvider.of<UserBloc>(context).dispatch(UserInit());
            return Center(child: Container(child: CircularProgressIndicator()));
          }
          if (state is UserLoaded) {
            return TopicsPage();
          }
          if (state is Loading) {
            return Center(child: Container(child: CircularProgressIndicator()));
          }
          if (state is UserNone) {
            return AuthenticationPage();
          }
        });
  }
}
