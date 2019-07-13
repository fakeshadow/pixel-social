import 'dart:math';

import 'package:flutter/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/CategoryBloc/CategoryBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorState.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserEvent.dart';
import 'package:pixel_flutter/blocs/VerticalTabBloc/VerticalTabBloc.dart';

import 'package:pixel_flutter/Views/HomePage.dart';
import 'package:pixel_flutter/Views/ProfilePage.dart';

import 'package:pixel_flutter/api/DataBase.dart';

void main() {
  runApp(PixelShare());
}

class PixelShare extends StatefulWidget {
  @override
  _PixelShareState createState() => _PixelShareState();
}

class _PixelShareState extends State<PixelShare> {
  UserBloc userBloc;
  ErrorBloc errorBloc;
  TalkBloc talkBloc;

  @override
  void initState() {
    errorBloc = ErrorBloc();
    userBloc = UserBloc();
    talkBloc = TalkBloc(errorBloc: errorBloc);

    DataBase.createDb();
    userBloc.dispatch(UserInit());
    talkBloc.dispatch(TalkInit());

    super.initState();
  }

  @override
  void dispose() {
    userBloc.dispose();
    errorBloc.dispose();
    talkBloc.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MultiBlocProvider(
        providers: [
          // bloc for handling error info
          BlocProvider<ErrorBloc>(builder: (context) => errorBloc),
          // bloc for handling user data
          BlocProvider<UserBloc>(builder: (context) => userBloc),
          // bloc for handling talks
          BlocProvider<TalkBloc>(builder: (context) => talkBloc),
          // bloc for handling categories data
          BlocProvider<CategoryBloc>(builder: (context) => CategoryBloc()),
          // bloc for handling vertical tab bar
          BlocProvider<VerticalTabBloc>(builder: (context) => VerticalTabBloc())
        ],
        child: MaterialApp(
          routes: {
            'profile': (context) => ProfilePage(),
            'home': (context) => HomePage(),
          },
          debugShowCheckedModeBanner: false,
          initialRoute: 'home',
          theme: ThemeData(
              brightness: Brightness.light,
              primarySwatch: Colors.blue,
              accentColor: Colors.deepPurple),
        ));
  }
}
