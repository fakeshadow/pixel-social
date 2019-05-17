import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/FloatingButtonBlocs.dart';
import 'package:pixel_flutter_web/blocs/CategoryBloc/CategoryBloc.dart';
import 'package:pixel_flutter_web/blocs/ErrorBloc/ErrorBloc.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/views/HomePage.dart';

void main() => runApp(MyApp());

class MyApp extends StatelessWidget {
  final ErrorBloc errorBloc = ErrorBloc();
  final UserBloc userBloc = UserBloc();
  final CategoryBloc categoryBloc = CategoryBloc();
  final FloatingButtonBloc floatingButtonBloc = FloatingButtonBloc();

  @override
  Widget build(BuildContext context) {
    userBloc.dispatch(UserInit());
    return BlocProviderTree(
      blocProviders: [
        // ToDo: change floating button bloc into generic visible controller bloc
        BlocProvider<FloatingButtonBloc>(bloc: floatingButtonBloc),
        BlocProvider<ErrorBloc>(bloc: errorBloc),
        BlocProvider<UserBloc>(bloc: userBloc),
        BlocProvider<CategoryBloc>(bloc: categoryBloc)
      ],
      child: MaterialApp(
        title: 'Pixel Flutter Web',
        debugShowCheckedModeBanner: false,
        theme: ThemeData(
          brightness: Brightness.light,
          primarySwatch: Colors.blue,
          accentColor: Colors.deepPurple,
          dividerColor: Colors.black,
        ),
        routes: {
          '/home': (context) => HomePage(title: 'PixelShare'),
        },
        home: HomePage(title: 'PixelShare'),
      ),
    );
  }
}
