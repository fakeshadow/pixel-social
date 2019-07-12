import 'package:flutter/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/CategoryBloc/CategoryBloc.dart';
import 'package:pixel_flutter/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserBloc.dart';
import 'package:pixel_flutter/blocs/UserBloc/UserEvent.dart';
import 'package:pixel_flutter/blocs/VerticalTabBloc/VerticalTabBloc.dart';

import 'package:pixel_flutter/Views/HomePage.dart';
import 'package:pixel_flutter/Views/ProfilePage.dart';

void main() {
  runApp(PixelShare());
}

class PixelShare extends StatelessWidget {
  final UserBloc userBloc = UserBloc();

  @override
  Widget build(BuildContext context) {
    userBloc.dispatch(UserInit());
    return MultiBlocProvider(
        providers: [
          // bloc for handling error info
          BlocProvider<ErrorBloc>(builder: (context) => ErrorBloc()),
          // bloc for handling user data
          BlocProvider<UserBloc>(builder: (context) => userBloc),
          // bloc for handling talks
          BlocProvider<TalkBloc>(builder: (context) => TalkBloc()),
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
