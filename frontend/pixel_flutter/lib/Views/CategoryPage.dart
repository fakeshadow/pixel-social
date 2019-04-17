import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/HorizontalTabBlocs.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter/components/Background/GeneralBackground.dart';
import 'package:pixel_flutter/components/NavigationBar/TabNavBar.dart';
import 'package:pixel_flutter/components/NavigationBar/HorizontalTab/HorizontalTabText.dart';
import 'package:pixel_flutter/components/Categories/CategoryList.dart';

class CategoryPage extends StatefulWidget {
  @override
  _CategoryPageState createState() => _CategoryPageState();
}

class _CategoryPageState extends State<CategoryPage> {
  HorizontalTabBloc _tabBloc;

  @override
  void initState() {
    _tabBloc = HorizontalTabBloc();
    super.initState();
  }

  @override
  void dispose() {
    _tabBloc.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final errorBloc = BlocProvider.of<ErrorBloc>(context);
    return Scaffold(
        bottomNavigationBar: TabNavBar(1),
        body: BlocListener(
            bloc: errorBloc,
            listener: (BuildContext context, ErrorState state) {
              if (state is ShowError) {
                Scaffold.of(context).showSnackBar(SnackBar(
                  backgroundColor: Colors.deepOrangeAccent,
                  content: Text(state.error),
                ));
              }
            },
            child: Scaffold(
                body: Stack(
              children: <Widget>[
                GeneralBackground(),
                Center(
                    child: BlocProvider(
                        bloc: _tabBloc,
                        child: Container(
                            height: 450,
                            child: BlocBuilder(
                                bloc: _tabBloc,
                                builder: (BuildContext context,
                                    HorizontalTabState tabState) {
                                  if (tabState is Selected) {
                                    return CardStack(
                                      selectedTabIndex: tabState.index,
                                    );
                                  }
                                }))))
              ],
            ))));
  }
}

class CardStack extends StatefulWidget {
  final int selectedTabIndex;

  CardStack({this.selectedTabIndex});

  @override
  _CardStackState createState() => _CardStackState();
}

class _CardStackState extends State<CardStack>
    with SingleTickerProviderStateMixin {
  AnimationController _animationController;
  Animation<double> _animationDouble;

  initAnimation() {
    _animationController.reset();
    _animationController.forward();
  }

  @override
  void initState() {
    _animationController = AnimationController(
        vsync: this, duration: Duration(milliseconds: 1000));
    _animationDouble =
        Tween<double>(begin: 0.0, end: 1.0).animate(_animationController);
    super.initState();
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: <Widget>[
        Positioned(
          left: 0.05,
          top: 10,
          bottom: 0,
          width: 70,
          child: Padding(
            padding: EdgeInsets.symmetric(vertical: 70.0),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                HorizontalTabText(
                  text: 'Media',
                  index: 0,
                ),
                HorizontalTabText(
                  text: 'Forum',
                  index: 1,
                ),
                HorizontalTabText(
                  text: 'Info',
                  index: 2,
                ),
              ],
            ),
          ),
        ),
        FutureBuilder(
          future: initAnimation(),
          builder: (context, snapshot) {
            return FadeTransition(
              opacity: _animationDouble,
              child: Padding(
                  padding: EdgeInsets.only(left: 60),
                  // ToDo: make separate class for list view on different tabs.
                  child: widget.selectedTabIndex == 0
                      ? CategoryList()
                      : widget.selectedTabIndex == 1
                          ? CategoryList()
                          : CategoryList()),
            );
          },
        ),
      ],
    );
  }
}

//                  if (state is AppStarted) {
//                    userBloc.dispatch(UserInit());
//                    return Center(
//                        child: Container(child: CircularProgressIndicator()));
//                  }
//                  if (state is UserLoaded) {
//                    return TopicsPage();
//                  }
//                  if (state is Loading) {
//                    return Center(
//                        child: Container(child: CircularProgressIndicator()));
//                  }
//                  if (state is UserLoggedOut) {
//                    return AuthenticationPage(
//                      type: 'login',
//                      username: state.username,
//                    );
//                  }
//                  if (state is UserNone) {
//                    return AuthenticationPage(type: 'register');
//                  }
//                  if (state is Failure) {
//                    errorBloc.dispatch(GetError(error: state.error));
//                  }
