import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter_web/components/GeneralBackground.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/components/BackToTopButton.dart';

class BasicLayout extends StatelessWidget {
  final Widget scrollView;
  final Widget sideMenu;
  final Function backToTop;

  BasicLayout({this.scrollView, this.sideMenu, this.backToTop});

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: BlocProvider.of<UserBloc>(context),
        builder: (context, userState) {
          return Scaffold(
            floatingActionButton: BackToTopButton(onPressed: backToTop),
            floatingActionButtonLocation: FloatingActionButtonLocation.endFloat,
            endDrawer: userState is UserLoaded ? UserDrawer() : null,
            body: BlocListener(
              bloc: BlocProvider.of<ErrorBloc>(context),
              listener: (context, state) {
                snackbarController(context, state);
              },
              child: Stack(
                alignment: Alignment.centerLeft,
                children: [
                  GeneralBackground(),
                  scrollView,
                  sideMenu,
                ],
              ),
            ),
          );
        });
  }

  snackbarController(BuildContext context, ErrorState state) {
    if (state is NoSnack) {
      Scaffold.of(context).hideCurrentSnackBar();
    } else if (state is ShowSuccess) {
      print(state.success);
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.green,
        content: Text(
          state.success,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    } else if (state is ShowError) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.deepOrangeAccent,
        content: Text(
          state.error,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    }
  }
}
