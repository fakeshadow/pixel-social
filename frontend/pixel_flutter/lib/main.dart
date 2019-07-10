import 'package:flutter/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/CategoryBlocs.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';

import 'package:pixel_flutter/Views/HomePage.dart';
import 'package:pixel_flutter/Views/ProfilePage.dart';

void main() {
  runApp(PixelShare());
}

class PixelShare extends StatelessWidget {
  final UserBloc userBloc = UserBloc();
  final ErrorBloc errorBloc = ErrorBloc();
  final CategoryBloc categoryBloc = CategoryBloc();

  @override
  Widget build(BuildContext context) {
    userBloc.dispatch(UserInit());
    return BlocProviderTree(
        blocProviders: [
          BlocProvider<ErrorBloc>(builder: (context) => errorBloc),
          BlocProvider<UserBloc>(builder: (context) => userBloc),
          BlocProvider<CategoryBloc>(builder: (context) => categoryBloc)
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
