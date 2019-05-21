import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/FloatingButtonBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicInputBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';
import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';

import 'package:pixel_flutter_web/components/BasicLayout.dart';
import 'package:pixel_flutter_web/components/BasicSliverPadding.dart';
import 'package:pixel_flutter_web/components/Lists/TopicsList.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';
import 'package:pixel_flutter_web/components/FloatingAppBar.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';
import 'package:pixel_flutter_web/views/InputPage.dart';

class HomePage extends StatefulWidget {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  TopicsBloc _topicsBloc;
  TopicInputBloc _topicInputBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    BlocProvider.of<FloatingButtonBloc>(context)
        .dispatch(ShowFloating(showFloating: false));
    BlocProvider.of<CategoryBloc>(context).dispatch(LoadCategories());
    _topicsBloc = TopicsBloc();
    _topicInputBloc = TopicInputBloc();
    _topicsBloc.dispatch(GetTopics(categoryId: 1));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicsBloc.dispose();
    _topicInputBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return WillPopScope(
        onWillPop: () => onWillPop(
            title: 'Leaving?', content: 'Do you want to exit the app?'),
        child: BasicLayout(
          scrollView: scrollView(),
          sideMenu: SideMenu(),
          backToTop: () => backTop(),
        ));
  }

  Widget scrollView() {
    return Scrollbar(
      child: CustomScrollView(
        controller: _scrollController,
        slivers: [
          FloatingAppBar(
              title: 'pixelshare example',
              onNewTopicButtonPressed: () => _showDialog()),
          BasicSliverPadding(sliver: TopicsList(topicsBloc: _topicsBloc))
        ],
      ),
    );
  }

  void _showDialog() async {
    await showDialog(
        context: context,
        builder: (context) {
          return InputPage(
              onWillPop: () => onWillPop(
                  title: 'Exiting input?', content: 'All input will be lost'),
              onCancelButtonPressed: () async {
                final result = await onWillPop(
                    title: 'Exit posting?',
                    content: 'All content will be lost');
                if (result == null || result == false) {
                  return false;
                } else {
                  Navigator.pop(context);
                }
              });
        });
  }

  void loadMore() {
    _topicsBloc.dispatch(GetTopics(categoryId: 1));
  }

  void backTop() {
    _scrollController.animateTo(0.0,
        duration: Duration(milliseconds: 300), curve: Curves.linear);
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      loadMore();
    }
    if (currentScroll > MediaQuery.of(context).size.height) {
      BlocProvider.of<FloatingButtonBloc>(context)
          .dispatch(ShowFloating(showFloating: true));
    }
    if (currentScroll < MediaQuery.of(context).size.height) {
      BlocProvider.of<FloatingButtonBloc>(context)
          .dispatch(ShowFloating(showFloating: false));
    }
  }

  Future<bool> onWillPop({String title, String content}) {
    return showDialog(
        context: context,
        builder: (context) => AlertDialog(
              title: Text(title),
              content: Text(content),
              actions: <Widget>[
                FlatButton(
                  onPressed: () => Navigator.pop(context, false),
                  child: Text(
                    'Cancel',
                    style: recoverButtonStyle,
                  ),
                ),
                RaisedButton(
                  color: primaryColor,
                  onPressed: () => Navigator.pop(context, true),
                  child: Text(
                    'Confirm',
                    style: submitButtonStyle.copyWith(fontSize: 16),
                  ),
                )
              ],
            ));
  }
}
